use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn crawl_schedule_sync_entries_are_persisted() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let now = Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();
    let subscription = subscription("crawl-schedule-sync-entry");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    project_crawl_targets(
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
