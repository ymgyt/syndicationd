//! End-to-end tests driving the full registry event chain against real
//! sqlite persistence and a local mock feed server:
//!
//! subscribe -> crawl target -> due derivation -> dispatch -> fetch ->
//! feed/entry projection -> timeline -> api event, and the crawl completion
//! feeding back into the crawl state (health and retry facts).

use std::time::Duration;

use synd_feed::types::FeedUrl;
use synd_persistence::sqlite::{SqliteDatabase, SqliteFeedRegistryDb};
use synd_registry::{
    CrawlStateStore, FeedRegistry, FeedRegistryConfig, FeedRegistryDb, FeedRegistryWorkerConfig,
    SubscribeFeedCommand, SubscriberId, api::ApiEvent, crawl::state::CrawlState,
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

/// Polls the crawl state until `condition` holds or the timeout passes.
async fn wait_for_crawl_state(
    db: &SqliteFeedRegistryDb,
    feed_url: &FeedUrl,
    condition: impl Fn(&CrawlState) -> bool,
) -> anyhow::Result<CrawlState> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let mut tx = db.begin().await?;
        let state = tx.load_crawl_state(feed_url).await?;
        drop(tx);

        if let Some(state) = &state
            && condition(state)
        {
            return Ok(state.clone());
        }
        if tokio::time::Instant::now() > deadline {
            anyhow::bail!("crawl state did not reach expected state: {state:?}");
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
    assert_eq!(changed.subscriber_id, subscriber_id);
    assert_eq!(changed.affected_feeds, vec![feed_url.clone()]);

    // Timeline items are queryable through the read side.
    let page = registry
        .list_timeline_entries(TimelineEntriesQuery {
            subscriber_id: subscriber_id.clone(),
            after: None,
            first: 10,
        })
        .await?;
    assert!(!page.nodes.is_empty(), "timeline should contain entries");

    // The finished crawl leaves its observation behind: a healthy state the
    // scheduler derives the next periodic crawl from.
    let state = wait_for_crawl_state(&db, &feed_url, |state| state.last.is_normal()).await?;
    assert_eq!(state.health.failure_streak.value(), 0);

    ct.cancel();
    drop(workers);
    Ok(())
}

#[tokio::test]
async fn failed_crawl_records_failure_state_for_retry() -> anyhow::Result<()> {
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

    // The failed crawl must leave the failure facts the scheduler derives
    // the retry backoff from.
    let state = wait_for_crawl_state(&db, &feed_url, |state| !state.last.is_normal()).await?;
    assert!(
        state.health.failure_streak.value() >= 1,
        "failure streak should grow"
    );

    ct.cancel();
    drop(workers);
    Ok(())
}
