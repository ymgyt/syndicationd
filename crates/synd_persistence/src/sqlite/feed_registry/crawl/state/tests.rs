use synd_feed::feed::service::{FeedConditionalFetch, FeedParseErrorKind};
use synd_registry::crawl::state::{
    CrawlHealth, CrawlStateError, FailureStreak, LastCrawlResult, UpsertCrawlStateCommand,
};

use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn crawl_state_round_trips_and_joins_into_due_input() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("crawl-state");
    let started_at = test_occurred_at();
    let finished_at = started_at + chrono::Duration::minutes(1);

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;
    project_crawl_targets(
        &db,
        vec![SubEvent::Subscribed(feed_subscribed_event(&subscription))],
    )
    .await?;

    let command = UpsertCrawlStateCommand::new(
        subscription.feed_url.clone(),
        LastCrawlResult::abnormal(
            started_at,
            finished_at,
            Some(FeedHttpStatus::new(200)),
            CrawlStateError::parse(FeedParseErrorKind::InvalidFeed),
            Some(finished_at + chrono::Duration::hours(1)),
        ),
        CrawlHealth {
            failure_streak: FailureStreak::new(2),
        },
        FeedConditionalFetch {
            etag: Some("etag-value".to_owned()),
            last_modified: Some("last-modified-value".to_owned()),
        },
    );
    let mut tx = db.begin().await?;
    tx.upsert_crawl_state(command).await?;

    let state = tx
        .load_crawl_state(&subscription.feed_url)
        .await?
        .expect("crawl state should be stored");
    assert_eq!(state.last.started_at, started_at);
    assert_eq!(state.last.finished_at, finished_at);
    assert!(!state.last.is_normal());
    assert_eq!(
        state.last.error.map(|error| error.kind),
        Some(CrawlStateErrorKind::Parse(FeedParseErrorKind::InvalidFeed))
    );
    assert_eq!(state.health.failure_streak.value(), 2);
    assert_eq!(state.conditional.etag.as_deref(), Some("etag-value"));

    // The scheduler sees the same facts joined into its due input.
    let input = tx
        .load_crawl_due_input(&subscription.feed_url)
        .await?
        .expect("active target should have a due input");
    assert_eq!(input.state.as_ref(), Some(&state));
    tx.commit().await?;
    Ok(())
}
