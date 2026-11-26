use synd_feed::types::{SubscriptionLabels, Text};

use std::sync::Arc;

use crate::{
    CreateTimelineCommand, FeedRegistry, FeedRegistryConfig, FeedRegistryError,
    SubscribeFeedCommand,
    event::{EventWorker, JournalWorker, TimelineCreatedEvent, Trigger},
    query::{Timeline, TimelineChange, TimelineChangesQuery, TimelinesQuery},
    sqlite::test_support::*,
};

#[tokio::test]
async fn subscription_catchup_reads_complete_entries_from_current_membership() -> anyhow::Result<()>
{
    let db = migrated_db().await?;
    let subscription = subscription("timeline-catchup");
    let registry = FeedRegistry::builder(db.clone(), FeedRegistryConfig::default()).build();
    let timeline = registry
        .create_timeline(CreateTimelineCommand {
            subscriber_id: subscription.subscriber_id.clone(),
            name: "all".into(),
        })
        .await?;
    let first = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry("timeline feed", "old entry", "entry-1"),
        0,
    )
    .await?;
    project_feed(&db, first).await?;
    let second = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry("timeline feed", "current entry", "entry-2"),
        1,
    )
    .await?;
    project_feed(&db, second).await?;
    store_subscription_in_db(&db, subscription.clone()).await?;

    let recorded = project_timeline(
        &db,
        TimelineProjInput::FeedSubscribed(feed_subscribed_event(&subscription)),
    )
    .await?;

    assert_eq!(recorded.types(), &[TimelineChangedEvent::TYPE]);
    let page = list_timeline_entries(&db, subscription.subscriber_id, timeline.timeline_id).await?;
    assert_eq!(page.nodes.len(), 1);
    assert_eq!(
        page.nodes[0].entry.title().map(Text::content),
        Some("current entry")
    );
    assert_eq!(
        page.nodes[0].feed_meta.feed.title().map(Text::content),
        Some("timeline feed")
    );
    Ok(())
}

#[tokio::test]
async fn create_after_crawl_projects_known_entries_from_the_owners_subscriptions()
-> anyhow::Result<()> {
    let db = migrated_db().await?;
    let registry = FeedRegistry::builder(db.clone(), FeedRegistryConfig::default()).build();
    let owner = subscriber_id();
    let url = feed_url("archive");
    registry
        .subscribe(SubscribeFeedCommand {
            subscriber_id: owner.clone(),
            feed_url: url.clone(),
            labels: SubscriptionLabels::default(),
            crawl_policy: None,
        })
        .await?;
    for (seq, title) in [(0, "old"), (1, "current")] {
        let crawl =
            record_fetched_crawl(&db, &url, rss_body_with_entry("feed", title, title), seq).await?;
        project_feed(&db, crawl).await?;
    }
    let mut worker = JournalWorker::new(
        db.clone(),
        TimelineProj::new(),
        Arc::new(TestClock(test_occurred_at())),
    );
    worker.react(Trigger::Startup).await?;
    assert_eq!(
        registry
            .list_timelines(TimelinesQuery {
                subscriber_id: owner.clone()
            })
            .await?,
        []
    );

    let first_timeline = registry
        .create_timeline(CreateTimelineCommand {
            subscriber_id: owner.clone(),
            name: "first".into(),
        })
        .await?
        .timeline_id;
    worker.react(Trigger::Wake).await?;
    let first_snapshot = list_timeline_entries(&db, owner.clone(), first_timeline).await?;
    assert_eq!(first_snapshot.nodes.len(), 2);

    let timeline_id = registry
        .create_timeline(CreateTimelineCommand {
            subscriber_id: owner.clone(),
            name: "archive".into(),
        })
        .await?
        .timeline_id;
    let before = list_timeline_entries(&db, owner.clone(), timeline_id).await?;
    assert!(before.nodes.is_empty());
    worker.react(Trigger::Wake).await?;

    assert_eq!(
        list_timeline_entries(&db, owner.clone(), first_timeline)
            .await?
            .seq,
        first_snapshot.seq
    );
    let changes_query = TimelineChangesQuery {
        subscriber_id: owner,
        timeline_id,
        since: before.seq,
        limit: 1,
    };
    let first = registry
        .list_timeline_changes(changes_query.clone())
        .await?;
    assert!(first.has_more);
    let second = registry
        .list_timeline_changes(TimelineChangesQuery {
            since: first.seq,
            ..changes_query.clone()
        })
        .await?;
    assert!(!second.has_more);
    let mut titles = Vec::new();
    for page in [&first, &second] {
        let [TimelineChange::Upsert(node)] = page.changes.as_slice() else {
            anyhow::bail!("expected one entry on each page");
        };
        titles.push(node.entry.title().unwrap().content());
    }
    titles.sort_unstable();
    assert_eq!(titles, ["current", "old"]);
    let other = SubscriberId::new("other");
    assert!(
        list_timeline_entries(&db, other.clone(), timeline_id)
            .await?
            .nodes
            .is_empty()
    );
    let other_changes = registry
        .list_timeline_changes(TimelineChangesQuery {
            subscriber_id: other,
            ..changes_query
        })
        .await?;
    assert!(other_changes.changes.is_empty());
    assert_eq!(other_changes.seq, 0);
    Ok(())
}

#[tokio::test]
async fn competing_creates_record_one_event_for_the_same_owner_and_name() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let registry = FeedRegistry::builder(db.clone(), FeedRegistryConfig::default()).build();
    let command = CreateTimelineCommand {
        subscriber_id: subscriber_id(),
        name: "all".into(),
    };
    let results = tokio::join!(
        registry.create_timeline(command.clone()),
        registry.create_timeline(command.clone())
    );
    let created = match results {
        (Ok(created), Err(FeedRegistryError::TimelineNameAlreadyExists { name }))
        | (Err(FeedRegistryError::TimelineNameAlreadyExists { name }), Ok(created))
            if name == command.name =>
        {
            created
        }
        other => anyhow::bail!("unexpected create results: {other:?}"),
    };
    let upper = registry
        .create_timeline(CreateTimelineCommand {
            name: "All".into(),
            ..command.clone()
        })
        .await?;
    registry
        .create_timeline(CreateTimelineCommand {
            subscriber_id: SubscriberId::new("other"),
            ..command.clone()
        })
        .await?;

    assert_eq!(
        registry
            .list_timelines(TimelinesQuery {
                subscriber_id: command.subscriber_id.clone()
            })
            .await?,
        [
            Timeline {
                id: upper.timeline_id,
                name: "All".into()
            },
            Timeline {
                id: created.timeline_id,
                name: "all".into()
            },
        ]
    );
    let mut tx = db.begin().await?;
    let batch = tx
        .read_after(
            &EventCursor::initial(ProcessorId::TimelineProjection),
            EventInterests::new([TimelineCreatedEvent::TYPE]),
        )
        .await?;
    assert_eq!(batch.events().len(), 3);
    assert_eq!(
        batch.events()[0].event(),
        &Event::TimelineCreated(TimelineCreatedEvent {
            timeline_id: created.timeline_id,
            subscriber_id: command.subscriber_id,
            name: command.name,
        })
    );
    Ok(())
}
