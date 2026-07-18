//! End-to-end tests driving the full registry event chain against real
//! sqlite persistence and a local mock feed server:
//!
//! subscribe -> crawl target -> crawl schedule -> dispatch -> fetch ->
//! feed/entry projection -> timeline -> api event, and the crawl completion
//! feeding back into the schedule (periodic advance and retry backoff).

use std::time::Duration;

use synd_feed::types::FeedUrl;
use synd_persistence::sqlite::{SqliteDatabase, SqliteFeedRegistryDb};
use synd_registry::{
    CrawlScheduleStore, FeedRegistry, FeedRegistryConfig, FeedRegistryDb, FeedRegistryWorkerConfig,
    SubscribeFeedCommand, SubscriberId,
    api::ApiEvent,
    crawl::schedule::{CrawlSchedule, DueReason},
    query::TimelineEntriesQuery,
};
use tokio_util::sync::CancellationToken;

async fn spawn_mock_feed_server() -> anyhow::Result<std::net::SocketAddr> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    synd_test::mock::spawn(listener);
    Ok(addr)
}

async fn migrated_registry_db(dir: &tempfile::TempDir) -> anyhow::Result<SqliteFeedRegistryDb> {
    let db = SqliteDatabase::create_or_open(dir.path().join("registry.db")).await?;
    db.migrate().await?;
    Ok(SqliteFeedRegistryDb::new(db))
}

fn registry_config() -> FeedRegistryConfig {
    FeedRegistryConfig {
        workers: FeedRegistryWorkerConfig::with_poll_interval(Duration::from_millis(200)),
        ..FeedRegistryConfig::default()
    }
}

fn subscribe_command(subscriber_id: &SubscriberId, feed_url: &FeedUrl) -> SubscribeFeedCommand {
    SubscribeFeedCommand {
        subscriber_id: subscriber_id.clone(),
        feed_url: feed_url.clone(),
        requirement: None,
        category: None,
        crawl_policy: None,
    }
}

/// Polls the schedule row until `condition` holds or the timeout passes.
async fn wait_for_schedule(
    db: &SqliteFeedRegistryDb,
    feed_url: &FeedUrl,
    condition: impl Fn(&CrawlSchedule) -> bool,
) -> anyhow::Result<CrawlSchedule> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let mut tx = db.begin().await?;
        let schedule = tx
            .load_schedule_sync_entry(feed_url)
            .await?
            .and_then(|entry| entry.schedule);
        drop(tx);

        if let Some(schedule) = &schedule
            && condition(schedule)
        {
            return Ok(schedule.clone());
        }
        if tokio::time::Instant::now() > deadline {
            anyhow::bail!("schedule did not reach expected state: {schedule:?}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn subscribe_flows_through_crawl_to_timeline_notification() -> anyhow::Result<()> {
    let mock_addr = spawn_mock_feed_server().await?;
    let dir = tempfile::tempdir()?;
    let db = migrated_registry_db(&dir).await?;
    let ct = CancellationToken::new();
    let (registry, workers) = FeedRegistry::start(db.clone(), registry_config(), ct.clone());

    let subscriber_id = SubscriberId::new("e2e-reader");
    let mut api_events = registry.subscribe_events(subscriber_id.clone());
    let feed_url = FeedUrl::parse(&format!("http://{mock_addr}/feed/twir_atom"))?;

    registry
        .subscribe(subscribe_command(&subscriber_id, &feed_url))
        .await?;

    // The whole chain must produce a subscriber-visible timeline notification.
    let event = tokio::time::timeout(Duration::from_secs(30), api_events.recv())
        .await?
        .map_err(|err| anyhow::anyhow!("api event recv failed: {err:?}"))?;
    let ApiEvent::TimelineChanged(changed) = event;
    assert_eq!(changed.timeline.subscriber_id, subscriber_id);
    assert_eq!(changed.affected_feeds, vec![feed_url.clone()]);

    // Timeline items are queryable through the read side.
    let page = registry
        .list_timeline_entries(TimelineEntriesQuery {
            subscriber_id: subscriber_id.clone(),
            feed_url: None,
            after: None,
            first: 10,
        })
        .await?;
    assert!(!page.nodes.is_empty(), "timeline should contain entries");

    // The finished crawl is reflected back into the schedule: the inflight
    // marker is cleared and the next periodic crawl is scheduled.
    let schedule = wait_for_schedule(&db, &feed_url, |schedule| {
        schedule.dispatched_at.is_none() && schedule.next_crawl_after.is_some()
    })
    .await?;
    assert_eq!(schedule.due_reason, DueReason::Periodic);

    ct.cancel();
    drop(workers);
    Ok(())
}

#[tokio::test]
async fn failed_crawl_schedules_retry() -> anyhow::Result<()> {
    let mock_addr = spawn_mock_feed_server().await?;
    let dir = tempfile::tempdir()?;
    let db = migrated_registry_db(&dir).await?;
    let ct = CancellationToken::new();
    let (registry, workers) = FeedRegistry::start(db.clone(), registry_config(), ct.clone());

    let subscriber_id = SubscriberId::new("e2e-retry-reader");
    let feed_url = FeedUrl::parse(&format!("http://{mock_addr}/feed/error/internal"))?;

    registry
        .subscribe(subscribe_command(&subscriber_id, &feed_url))
        .await?;

    // The failed crawl must feed back into the schedule as a retry with the
    // inflight marker cleared.
    let schedule = wait_for_schedule(&db, &feed_url, |schedule| {
        schedule.dispatched_at.is_none() && schedule.due_reason == DueReason::Retry
    })
    .await?;
    assert!(
        schedule.next_crawl_after.is_some(),
        "retry should schedule a next crawl"
    );

    ct.cancel();
    drop(workers);
    Ok(())
}
