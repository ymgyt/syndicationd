use synd_registry::crawl::schedule::{CompleteDispatchCommand, DueReason};

use crate::sqlite::feed_registry::test_support::*;

async fn activated_target_entry(
    db: &SqliteFeedRegistryDb,
    name: &str,
) -> anyhow::Result<(Subscription, DateTime<Utc>)> {
    let subscription = subscription(name);

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    project_crawl_targets(
        db,
        vec![SubEvent::Subscribed(feed_subscribed_event(&subscription))],
    )
    .await?;

    let mut tx = db.begin().await?;
    let entry = tx
        .load_schedule_sync_entry(&subscription.feed_url)
        .await?
        .expect("schedule sync entry should exist");
    tx.commit().await?;
    Ok((subscription, entry.target.target_updated_at))
}

fn upsert_command(
    subscription: &Subscription,
    target_updated_at: DateTime<Utc>,
    next_crawl_after: Option<DateTime<Utc>>,
    due_reason: DueReason,
    synced_at: DateTime<Utc>,
) -> UpsertCrawlScheduleCommand {
    UpsertCrawlScheduleCommand::builder()
        .feed_url(subscription.feed_url.clone())
        .target_updated_at(target_updated_at)
        .maybe_next_crawl_after(next_crawl_after)
        .due_reason(due_reason)
        .synced_at(synced_at)
        .build()
}

#[tokio::test]
async fn crawl_schedule_roundtrips_with_due_reason() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let now = Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();
    let (subscription, target_updated_at) =
        activated_target_entry(&db, "crawl-schedule-roundtrip").await?;

    let mut tx = db.begin().await?;
    tx.upsert_schedule(upsert_command(
        &subscription,
        target_updated_at,
        Some(now),
        DueReason::Manual,
        now,
    ))
    .await?;
    tx.commit().await?;

    let mut tx = db.begin().await?;
    let entry = tx
        .load_schedule_sync_entry(&subscription.feed_url)
        .await?
        .expect("schedule sync entry should exist");
    let schedule = entry.schedule.expect("schedule row should exist");
    assert_eq!(schedule.next_crawl_after, Some(now));
    assert_eq!(schedule.due_reason, DueReason::Manual);
    assert_eq!(schedule.dispatched_at, None);
    Ok(())
}

#[tokio::test]
async fn dispatch_lifecycle_marks_and_completes() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let now = Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();
    let stale_before = now - chrono::Duration::minutes(5);
    let (subscription, target_updated_at) =
        activated_target_entry(&db, "crawl-schedule-dispatch-lifecycle").await?;

    let mut tx = db.begin().await?;
    tx.upsert_schedule(upsert_command(
        &subscription,
        target_updated_at,
        Some(now),
        DueReason::Periodic,
        now,
    ))
    .await?;

    // due and not inflight -> dispatchable
    let candidates = tx.list_dispatchable(now, stale_before, 10).await?;
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].feed_url, subscription.feed_url);
    assert_eq!(candidates[0].due_reason, DueReason::Periodic);

    // inflight -> not dispatchable until stale
    tx.mark_dispatched(std::slice::from_ref(&subscription.feed_url), now)
        .await?;
    assert!(
        tx.list_dispatchable(now, stale_before, 10)
            .await?
            .is_empty()
    );

    // upsert preserves the inflight marker
    tx.upsert_schedule(upsert_command(
        &subscription,
        target_updated_at,
        Some(now),
        DueReason::Manual,
        now,
    ))
    .await?;
    let entry = tx
        .load_schedule_sync_entry(&subscription.feed_url)
        .await?
        .expect("schedule sync entry should exist");
    assert_eq!(
        entry.schedule.expect("schedule row").dispatched_at,
        Some(now)
    );

    // next wake is the stale deadline of the inflight row
    let stale_timeout = Duration::from_mins(5);
    assert_eq!(
        tx.next_dispatch_at(now, stale_timeout).await?,
        Some(now + chrono::Duration::seconds(300))
    );

    // finished crawl clears the marker and schedules the next due
    let next = now + chrono::Duration::hours(1);
    tx.complete_dispatch(
        CompleteDispatchCommand::builder()
            .feed_url(subscription.feed_url.clone())
            .target_updated_at(target_updated_at)
            .next_crawl_after(next)
            .due_reason(DueReason::Periodic)
            .synced_at(now)
            .build(),
    )
    .await?;
    let entry = tx
        .load_schedule_sync_entry(&subscription.feed_url)
        .await?
        .expect("schedule sync entry should exist");
    let schedule = entry.schedule.expect("schedule row");
    assert_eq!(schedule.dispatched_at, None);
    assert_eq!(schedule.next_crawl_after, Some(next));
    assert_eq!(tx.next_dispatch_at(now, stale_timeout).await?, Some(next));

    tx.commit().await?;
    Ok(())
}

#[tokio::test]
async fn dispatchable_orders_by_reason_priority() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let now = Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();
    let stale_before = now - chrono::Duration::minutes(5);
    let (periodic, periodic_updated_at) =
        activated_target_entry(&db, "crawl-schedule-order-periodic").await?;
    let (manual, manual_updated_at) =
        activated_target_entry(&db, "crawl-schedule-order-manual").await?;
    let (retry, retry_updated_at) =
        activated_target_entry(&db, "crawl-schedule-order-retry").await?;

    let mut tx = db.begin().await?;
    // periodic due earliest, but manual and retry take precedence
    tx.upsert_schedule(upsert_command(
        &periodic,
        periodic_updated_at,
        Some(now - chrono::Duration::minutes(3)),
        DueReason::Periodic,
        now,
    ))
    .await?;
    tx.upsert_schedule(upsert_command(
        &manual,
        manual_updated_at,
        Some(now - chrono::Duration::minutes(1)),
        DueReason::Manual,
        now,
    ))
    .await?;
    tx.upsert_schedule(upsert_command(
        &retry,
        retry_updated_at,
        Some(now - chrono::Duration::minutes(2)),
        DueReason::Retry,
        now,
    ))
    .await?;

    let candidates = tx.list_dispatchable(now, stale_before, 10).await?;
    let feed_urls = candidates
        .iter()
        .map(|candidate| candidate.feed_url.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        feed_urls,
        vec![
            manual.feed_url.clone(),
            retry.feed_url.clone(),
            periodic.feed_url.clone()
        ]
    );
    tx.commit().await?;
    Ok(())
}

#[tokio::test]
async fn stale_dispatch_becomes_dispatchable_again() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let dispatched_at = Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();
    let now = dispatched_at + chrono::Duration::minutes(10);
    let stale_before = now - chrono::Duration::minutes(5);
    let (subscription, target_updated_at) =
        activated_target_entry(&db, "crawl-schedule-stale-dispatch").await?;

    let mut tx = db.begin().await?;
    tx.upsert_schedule(upsert_command(
        &subscription,
        target_updated_at,
        Some(dispatched_at),
        DueReason::Periodic,
        dispatched_at,
    ))
    .await?;
    tx.mark_dispatched(std::slice::from_ref(&subscription.feed_url), dispatched_at)
        .await?;

    let candidates = tx.list_dispatchable(now, stale_before, 10).await?;
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].feed_url, subscription.feed_url);
    tx.commit().await?;
    Ok(())
}
