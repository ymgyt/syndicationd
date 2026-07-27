pub(crate) use std::time::Duration;

pub(crate) use chrono::{DateTime, TimeZone, Utc};
pub(crate) use sqlx::Row;
pub(crate) use synd_feed::feed::service::FeedHttpStatus;
pub(crate) use synd_feed::types::FeedUrl;
pub(crate) use synd_registry::{
    FeedSubscriptionAttrs, RegistryDbResult, SubscriberId, Subscription, SubscriptionKey,
    crawl::{
        blob::PutBlobCommand,
        job::CrawlJobId,
        policy::{CrawlPolicy, PollingInterval, PollingPolicy},
        state::CrawlStateErrorKind,
        target_list::{CrawlTargetProj, CrawlTargetProjInput, CrawlTargetState},
    },
    db::{
        BlobDb, CommitTx, CrawlStateDb, CrawlTargetDb, FeedDb, FeedRegistryDb, SubscriptionDb,
        TimelineDb,
    },
    event::{
        CrawlJobFinishedEvent, CrawlTargetActivatedEvent, CrawlTargetDeactivatedEvent,
        CrawlTargetPolicyChangedEvent, EntryDiscoveredEvent, Event, EventCursor, EventCursorPos,
        EventInterests, EventJournal, EventRecorder, FeedSubscribedEvent, FeedUnsubscribedEvent,
        InputBatch, ProcessorId, Projector, RecordedEvents, RegistryEvent, SubEvent,
        SubscriptionChangedEvent, TimelineChangedEvent,
    },
    feed::{FeedProj, FeedProjInput},
    query::{SubscriptionsQuery, TimelineEntriesPage, TimelineEntriesQuery},
    timeline::{TimelineProj, TimelineProjInput},
};
pub(crate) use synd_support::time::Clock;

use super::{error::IntoDbResult, feed};

pub(crate) use super::{SqliteFeedRegistryDb, SqliteRegistryTx};
pub(crate) use crate::sqlite::SqliteDatabase;

pub(crate) async fn migrated_db() -> anyhow::Result<SqliteFeedRegistryDb> {
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
    Subscription {
        subscriber_id,
        feed_url: feed_url(path),
        requirement: None,
        category: None,
        crawl_policy,
        subscribed_at: Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap(),
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
    tx.upsert_subscription(&key, attrs, subscription.subscribed_at)
        .await
}

pub(crate) async fn store_feed(
    tx: &mut SqliteRegistryTx<'_>,
    feed_url: &FeedUrl,
) -> RegistryDbResult<()> {
    feed::upsert_pk(&mut tx.tx, feed_url).await.map(|_| ()).db()
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

/// Registers the feed, stores the fetched body as a blob, and returns the
/// `CrawlJobFinished` fact a crawl worker would have recorded.
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
    store_feed(&mut tx, feed_url).await?;
    let body_blob = tx.put_blob(PutBlobCommand::new(body, finished_at)).await?;
    let event = CrawlJobFinishedEvent::new(
        CrawlJobId::generate(),
        feed_url.clone(),
        started_at,
        Some(body_blob),
    );
    tx.commit().await?;
    Ok(event)
}

pub(crate) async fn project_feed(
    db: &SqliteFeedRegistryDb,
    event: CrawlJobFinishedEvent,
) -> anyhow::Result<RecordedEvents> {
    let mut tx = db.begin().await?;
    let mut proj = FeedProj::new();
    let input = FeedProjInput::new(event, test_occurred_at());
    let events =
        <FeedProj as Projector<SqliteFeedRegistryDb>>::project(&mut proj, &mut tx, input).await?;
    let recorded = record_generated_events(&mut tx, events).await?;
    tx.commit().await?;
    Ok(recorded)
}

pub(crate) async fn project_timeline(
    db: &SqliteFeedRegistryDb,
    input: TimelineProjInput,
) -> anyhow::Result<RecordedEvents> {
    let mut tx = db.begin().await?;
    let mut proj = TimelineProj::new();
    let events =
        <TimelineProj as Projector<SqliteFeedRegistryDb>>::project(&mut proj, &mut tx, input)
            .await?;
    let recorded = record_generated_events(&mut tx, events).await?;
    tx.commit().await?;
    Ok(recorded)
}

pub(crate) async fn list_timeline_entries(
    db: &SqliteFeedRegistryDb,
    subscriber_id: SubscriberId,
) -> anyhow::Result<TimelineEntriesPage> {
    let mut tx = db.begin().await?;
    let page = tx
        .list_timeline_entries(TimelineEntriesQuery {
            subscriber_id,
            after: None,
            first: 10,
        })
        .await?;
    tx.commit().await?;
    Ok(page)
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
      <link>https://example.com/entry/{entry_guid}</link>
      <guid>{entry_guid}</guid>
    </item>
  </channel>
</rss>"#
    )
    .into_bytes()
}
