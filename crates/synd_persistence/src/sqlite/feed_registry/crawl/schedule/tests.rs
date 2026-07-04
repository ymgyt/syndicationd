use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn crawl_schedule_sync_entries_are_persisted() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let now = Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();
    let subscription = subscription("crawl-schedule-sync-entry");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    reconcile_crawl_targets(
        &db,
        vec![SubEvent::Subscribed(feed_subscribed_event(&subscription))],
    )
    .await?;

    let mut tx = db.begin().await?;
    let entries = tx.list_schedule_sync_entries(10).await?;
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.target.feed_url, subscription.feed_url);
    assert!(entry.schedule.is_none());

    tx.upsert_schedule(UpsertCrawlScheduleCommand::new(
        subscription.feed_url.clone(),
        entry.target.target_updated_at,
        Some(now),
        now,
    ))
    .await?;
    tx.commit().await?;

    let mut tx = db.begin().await?;
    let entries = tx.list_schedule_sync_entries(10).await?;
    assert!(entries.is_empty());
    Ok(())
}

#[tokio::test]
async fn next_scheduled_due_returns_nearest_future_due() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let now = Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();
    let due = subscription("crawl-schedule-next-due-due");
    let next = subscription("crawl-schedule-next-due-next");
    let later = subscription("crawl-schedule-next-due-later");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, due.clone()).await?;
    store_subscription(&mut tx, next.clone()).await?;
    store_subscription(&mut tx, later.clone()).await?;
    tx.commit().await?;

    reconcile_crawl_targets(
        &db,
        vec![
            SubEvent::Subscribed(feed_subscribed_event(&due)),
            SubEvent::Subscribed(feed_subscribed_event(&next)),
            SubEvent::Subscribed(feed_subscribed_event(&later)),
        ],
    )
    .await?;

    let next_due = now + chrono::Duration::minutes(5);
    let later_due = now + chrono::Duration::minutes(10);
    let mut tx = db.begin().await?;
    let due_entry = tx
        .load_schedule_sync_entry(&due.feed_url)
        .await?
        .expect("due schedule sync entry should exist");
    let next_entry = tx
        .load_schedule_sync_entry(&next.feed_url)
        .await?
        .expect("next schedule sync entry should exist");
    let later_entry = tx
        .load_schedule_sync_entry(&later.feed_url)
        .await?
        .expect("later schedule sync entry should exist");

    tx.upsert_schedule(UpsertCrawlScheduleCommand::new(
        due.feed_url.clone(),
        due_entry.target.target_updated_at,
        Some(now),
        now,
    ))
    .await?;
    tx.upsert_schedule(UpsertCrawlScheduleCommand::new(
        next.feed_url.clone(),
        next_entry.target.target_updated_at,
        Some(next_due),
        now,
    ))
    .await?;
    tx.upsert_schedule(UpsertCrawlScheduleCommand::new(
        later.feed_url.clone(),
        later_entry.target.target_updated_at,
        Some(later_due),
        now,
    ))
    .await?;

    assert_eq!(tx.next_scheduled_due(now).await?, Some(next_due));
    assert_eq!(tx.next_scheduled_due(next_due).await?, Some(later_due));
    tx.commit().await?;
    Ok(())
}
