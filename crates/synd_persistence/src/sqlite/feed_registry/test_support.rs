pub(crate) use std::time::Duration;

pub(crate) use chrono::{DateTime, TimeZone, Utc};
pub(crate) use sqlx::Row;
pub(crate) use synd_feed::feed::service::{
    FeedFetchFailure, FeedFetchFailureKind, FeedFetchOutcome, FeedHttpResponse, FeedHttpStatus,
    FeedResponseBody, FeedResponseHeaders, FeedService, FetchedFeed,
};
pub(crate) use synd_feed::types::{EntryId, FeedUrl};
pub(crate) use synd_registry::{
    BlobStore, CommitTx, CrawlResultStore, CrawlScheduleStore, CrawlTargetStore, FeedRegistryDb,
    FeedSubscriptionAttrs, RegistryDbError, RegistryDbResult, SubscriberId, Subscription,
    SubscriptionKey, SubscriptionStore, TimelineStore,
    crawl::completion::CrawlCompletionRecorder,
    crawl::{
        blob::PutBlobCommand,
        job::{CrawlJob, CrawlJobId, CrawlJobTrigger},
        policy::{CrawlPolicy, PollingInterval, PollingPolicy},
        result::CrawlStateErrorKind,
        schedule::UpsertCrawlScheduleCommand,
        target_list::{CrawlTargetProj, CrawlTargetProjInput, CrawlTargetState},
    },
    entry::{EntryProj, EntryProjInput},
    event::{
        CrawlJobFinishedEvent, CrawlTargetActivatedEvent, CrawlTargetDeactivatedEvent,
        CrawlTargetPolicyChangedEvent, EntryChangedEvent, EntryDiscoveredEvent, Event, EventCursor,
        EventCursorPos, EventInterests, EventJournal, EventRecorder, FeedChangedEvent,
        FeedDiscoveredEvent, FeedSubscribedEvent, FeedUnsubscribedEvent, InputBatch, ProcessorId,
        Projector, RecordedEvents, RegistryEvent, SubEvent, SubscriptionChangedEvent,
        TimelineChangedEvent,
    },
    feed::FeedProj,
    query::{SubscriptionsQuery, TimelineItemsPage, TimelineItemsQuery},
    timeline::{TimelineProj, TimelineProjInput},
};
pub(crate) use synd_support::time::Clock;

use super::{error::IntoDbResult, feed_endpoint};

pub(crate) use super::{SqliteFeedRegistryDb, SqliteRegistryTx};
pub(crate) use crate::sqlite::SqliteDatabase;

pub(crate) async fn migrated_db() -> Result<SqliteFeedRegistryDb, RegistryDbError> {
    let db = SqliteDatabase::in_memory().await?;
    db.migrate().await?;
    Ok(SqliteFeedRegistryDb::new(db))
}

pub(crate) fn test_occurred_at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 6, 8, 12, 0, 0).unwrap()
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TestClock(pub(crate) DateTime<Utc>);

impl Clock for TestClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

pub(crate) async fn record_generated_events(
    tx: &mut SqliteRegistryTx<'_>,
    events: Vec<Event>,
) -> RegistryDbResult<RecordedEvents> {
    let mut recorded = RecordedEvents::with_capacity(events.len());
    let clock = TestClock(test_occurred_at());
    EventRecorder::new(tx, &mut recorded, &clock)
        .record_all(events)
        .await?;
    Ok(recorded)
}

pub(crate) fn timeline_feed_subscribed(event: FeedSubscribedEvent) -> TimelineProjInput {
    TimelineProjInput::FeedSubscribed {
        event,
        occurred_at: test_occurred_at(),
    }
}

pub(crate) fn timeline_feed_unsubscribed(event: FeedUnsubscribedEvent) -> TimelineProjInput {
    TimelineProjInput::FeedUnsubscribed {
        event,
        occurred_at: test_occurred_at(),
    }
}

pub(crate) fn timeline_entry_discovered(event: EntryDiscoveredEvent) -> TimelineProjInput {
    TimelineProjInput::EntryDiscovered {
        event,
        occurred_at: test_occurred_at(),
    }
}

pub(crate) fn timeline_entry_changed(event: EntryChangedEvent) -> TimelineProjInput {
    TimelineProjInput::EntryChanged {
        event,
        occurred_at: test_occurred_at(),
    }
}

pub(crate) fn feed_url(path: &str) -> FeedUrl {
    FeedUrl::parse(&format!("https://example.com/{path}.xml")).unwrap()
}

pub(crate) fn subscriber_id() -> SubscriberId {
    SubscriberId::new("local")
}

pub(crate) fn interval(seconds: u64) -> PollingInterval {
    PollingInterval::try_from(Duration::from_secs(seconds)).unwrap()
}

pub(crate) fn subscription(path: &str) -> Subscription {
    subscription_with(subscriber_id(), path, CrawlPolicy::interval(interval(3600)))
}

pub(crate) fn subscription_with(
    subscriber_id: SubscriberId,
    path: &str,
    crawl_policy: CrawlPolicy,
) -> Subscription {
    let now = Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap();
    Subscription {
        subscriber_id,
        feed_url: feed_url(path),
        requirement: None,
        category: None,
        crawl_policy,
        created_at: now,
        updated_at: now,
    }
}

pub(crate) fn subscription_key(subscription: &Subscription) -> SubscriptionKey {
    SubscriptionKey::new(
        subscription.subscriber_id.clone(),
        subscription.feed_url.clone(),
    )
}

pub(crate) fn subscription_attrs(subscription: &Subscription) -> FeedSubscriptionAttrs {
    FeedSubscriptionAttrs {
        requirement: subscription.requirement,
        category: subscription.category.clone(),
        crawl_policy: subscription.crawl_policy,
    }
}

pub(crate) fn feed_subscribed_event(subscription: &Subscription) -> FeedSubscribedEvent {
    FeedSubscribedEvent::new(
        subscription_key(subscription),
        subscription_attrs(subscription),
    )
}

pub(crate) fn subscription_changed_event(subscription: &Subscription) -> SubscriptionChangedEvent {
    SubscriptionChangedEvent::new(
        subscription_key(subscription),
        subscription_attrs(subscription),
    )
}

pub(crate) async fn store_subscription(
    tx: &mut SqliteRegistryTx<'_>,
    subscription: Subscription,
) -> RegistryDbResult<()> {
    let key = subscription_key(&subscription);
    let attrs = subscription_attrs(&subscription);
    tx.upsert_subscription(&key, attrs, subscription.created_at)
        .await
}

pub(crate) async fn store_feed_endpoint(
    tx: &mut SqliteRegistryTx<'_>,
    feed_url: &FeedUrl,
    now: DateTime<Utc>,
) -> RegistryDbResult<()> {
    feed_endpoint::upsert(&mut tx.tx, feed_url, now, now)
        .await
        .map(|_| ())
        .db()
}

pub(crate) async fn store_subscription_in_db(
    db: &SqliteFeedRegistryDb,
    subscription: Subscription,
) -> anyhow::Result<()> {
    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) fn subscribed_event(path: &str) -> Event {
    let subscription = subscription(path);
    feed_subscribed_event(&subscription).into()
}

pub(crate) fn changed_event(path: &str) -> Event {
    let subscription = subscription(path);
    subscription_changed_event(&subscription).into()
}

pub(crate) fn subscription_lifecycle_interests() -> EventInterests {
    EventInterests::new([
        FeedSubscribedEvent::TYPE,
        SubscriptionChangedEvent::TYPE,
        FeedUnsubscribedEvent::TYPE,
    ])
}

pub(crate) fn crawl_target_interests() -> EventInterests {
    EventInterests::new([
        CrawlTargetActivatedEvent::TYPE,
        CrawlTargetPolicyChangedEvent::TYPE,
        CrawlTargetDeactivatedEvent::TYPE,
    ])
}

pub(crate) fn timeline_interests() -> EventInterests {
    EventInterests::new([TimelineChangedEvent::TYPE])
}

pub(crate) async fn project_crawl_targets(
    db: &SqliteFeedRegistryDb,
    events: Vec<SubEvent>,
) -> anyhow::Result<()> {
    let mut projection = CrawlTargetProj::new();
    let mut tx = db.begin().await?;
    let inputs = events
        .into_iter()
        .map(|event| CrawlTargetProjInput::new(event, test_occurred_at()))
        .collect();
    let generated = <CrawlTargetProj as Projector<SqliteFeedRegistryDb>>::project_batch(
        &mut projection,
        &mut tx,
        InputBatch::new(inputs),
    )
    .await?;
    record_generated_events(&mut tx, generated).await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn read_crawl_target_events(
    db: &SqliteFeedRegistryDb,
) -> anyhow::Result<Vec<Event>> {
    let mut tx = db.begin().await?;
    let cursor = tx.load_cursor(ProcessorId::CrawlTargetProjection).await?;
    let batch = tx.read_after(&cursor, crawl_target_interests()).await?;
    tx.commit().await?;

    let events = batch
        .into_events()
        .into_iter()
        .map(synd_registry::event::JournaledEvent::into_event)
        .collect::<Vec<_>>();
    Ok(events)
}

pub(crate) async fn read_timeline_events(db: &SqliteFeedRegistryDb) -> anyhow::Result<Vec<Event>> {
    let mut tx = db.begin().await?;
    let cursor = tx.load_cursor(ProcessorId::ApiEventPublisher).await?;
    let batch = tx.read_after(&cursor, timeline_interests()).await?;
    tx.commit().await?;

    let events = batch
        .into_events()
        .into_iter()
        .map(synd_registry::event::JournaledEvent::into_event)
        .collect::<Vec<_>>();
    Ok(events)
}

pub(crate) async fn record_fetched_crawl(
    db: &SqliteFeedRegistryDb,
    feed_url: &FeedUrl,
    body: Vec<u8>,
    seq: i64,
) -> anyhow::Result<CrawlJobFinishedEvent> {
    let started_at =
        Utc.with_ymd_and_hms(2026, 6, 8, 12, 0, 0).unwrap() + chrono::Duration::minutes(seq * 3);
    let finished_at = started_at + chrono::Duration::minutes(2);
    let mut tx = db.begin().await?;
    store_feed_endpoint(&mut tx, feed_url, started_at).await?;
    let job = CrawlJob::new(
        CrawlJobId::generate(),
        feed_url.clone(),
        CrawlJobTrigger::PeriodicDue,
        started_at,
    );
    let event = CrawlJobFinishedEvent::new(job.job_id.clone(), job.feed_url.clone());
    let outcome = fetched_outcome(feed_url.clone(), body, finished_at)?;
    let (_record, events) = CrawlCompletionRecorder::new(&mut tx)
        .record(job, outcome, None, finished_at)
        .await?;
    record_generated_events(&mut tx, events).await?;
    tx.commit().await?;
    Ok(event)
}

pub(crate) async fn project_feed(
    db: &SqliteFeedRegistryDb,
    event: CrawlJobFinishedEvent,
) -> anyhow::Result<RecordedEvents> {
    let mut tx = db.begin().await?;
    let mut proj = FeedProj::new();
    let events =
        <FeedProj as Projector<SqliteFeedRegistryDb>>::project(&mut proj, &mut tx, event).await?;
    let recorded = record_generated_events(&mut tx, events).await?;
    tx.commit().await?;
    Ok(recorded)
}

pub(crate) async fn project_entries(
    db: &SqliteFeedRegistryDb,
    input: EntryProjInput,
) -> anyhow::Result<RecordedEvents> {
    let mut tx = db.begin().await?;
    let mut proj = EntryProj::new();
    let events =
        <EntryProj as Projector<SqliteFeedRegistryDb>>::project(&mut proj, &mut tx, input).await?;
    let recorded = record_generated_events(&mut tx, events).await?;
    tx.commit().await?;
    Ok(recorded)
}

pub(crate) async fn project_timeline(
    db: &SqliteFeedRegistryDb,
    input: TimelineProjInput,
) -> anyhow::Result<RecordedEvents> {
    project_timeline_batch(db, vec![input]).await
}

pub(crate) async fn project_timeline_batch(
    db: &SqliteFeedRegistryDb,
    inputs: Vec<TimelineProjInput>,
) -> anyhow::Result<RecordedEvents> {
    let mut tx = db.begin().await?;
    let mut proj = TimelineProj::new();
    let events = <TimelineProj as Projector<SqliteFeedRegistryDb>>::project_batch(
        &mut proj,
        &mut tx,
        InputBatch::new(inputs),
    )
    .await?;
    let recorded = record_generated_events(&mut tx, events).await?;
    tx.commit().await?;
    Ok(recorded)
}

pub(crate) async fn entry_current_row(db: &SqliteFeedRegistryDb) -> anyhow::Result<(String, i64)> {
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        r#"
            SELECT current_content_json, current_source_result_pk
            FROM entry
            "#,
    )
    .fetch_one(&mut *tx.tx)
    .await?;
    let content_json = row.try_get::<String, _>("current_content_json")?;
    let source_result_pk = row.try_get::<i64, _>("current_source_result_pk")?;
    tx.commit().await?;
    Ok((content_json, source_result_pk))
}

pub(crate) async fn current_entry_id(db: &SqliteFeedRegistryDb) -> anyhow::Result<EntryId> {
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        r#"
            SELECT entry_id
            FROM entry
            "#,
    )
    .fetch_one(&mut *tx.tx)
    .await?;
    let entry_id = row.try_get::<String, _>("entry_id")?;
    tx.commit().await?;
    EntryId::parse(entry_id).map_err(Into::into)
}

pub(crate) async fn list_timeline_items(
    db: &SqliteFeedRegistryDb,
    subscriber_id: SubscriberId,
) -> anyhow::Result<TimelineItemsPage> {
    let mut tx = db.begin().await?;
    let page = tx
        .list_timeline_items(TimelineItemsQuery {
            subscriber_id,
            feed_url: None,
            after: None,
            first: 10,
        })
        .await?;
    tx.commit().await?;
    Ok(page)
}

pub(crate) fn fetched_outcome(
    feed_url: FeedUrl,
    body: Vec<u8>,
    fetched_at: DateTime<Utc>,
) -> anyhow::Result<FeedFetchOutcome> {
    let feed = FeedService::parse_feed(feed_url.clone(), body.as_slice())?;
    Ok(FeedFetchOutcome::Fetched(Box::new(FetchedFeed {
        body: FeedResponseBody {
            response: FeedHttpResponse {
                requested_url: feed_url.clone(),
                response_url: feed_url,
                status: FeedHttpStatus::new(200),
                headers: FeedResponseHeaders::default(),
                fetched_at,
            },
            bytes: body,
        },
        feed,
    })))
}

pub(crate) fn rss_body(title: &str) -> Vec<u8> {
    rss_body_with_entry(title, "first entry", "entry-1")
}

pub(crate) fn rss_body_with_entry(
    feed_title: &str,
    entry_title: &str,
    entry_guid: &str,
) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0">
  <channel>
    <title>{feed_title}</title>
    <link>https://example.com/</link>
    <description>example feed</description>
    <item>
      <title>{entry_title}</title>
      <link>https://example.com/entry/1</link>
      <guid>{entry_guid}</guid>
    </item>
  </channel>
</rss>"#
    )
    .into_bytes()
}

pub(crate) fn rss_body_with_entry_published_at(
    feed_title: &str,
    entry_title: &str,
    entry_guid: &str,
    published_at: DateTime<Utc>,
) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0">
  <channel>
    <title>{feed_title}</title>
    <link>https://example.com/</link>
    <description>example feed</description>
    <item>
      <title>{entry_title}</title>
      <link>https://example.com/entry/1</link>
      <guid>{entry_guid}</guid>
      <pubDate>{}</pubDate>
    </item>
  </channel>
</rss>"#,
        published_at.to_rfc2822()
    )
    .into_bytes()
}
