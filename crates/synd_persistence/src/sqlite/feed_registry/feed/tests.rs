use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn feed_projection_records_discovered_unchanged_and_changed() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let feed_url = feed_url("feed-projection");
    let first_body = rss_body("first title");

    let first = record_fetched_crawl(&db, &feed_url, first_body.clone(), 0).await?;
    let recorded = project_feed(&db, first).await?;
    assert_eq!(recorded.types(), &[FeedDiscoveredEvent::TYPE]);

    let second = record_fetched_crawl(&db, &feed_url, first_body, 1).await?;
    let recorded = project_feed(&db, second).await?;
    assert!(recorded.is_empty());

    let third = record_fetched_crawl(&db, &feed_url, rss_body("changed title"), 2).await?;
    let recorded = project_feed(&db, third).await?;
    assert_eq!(recorded.types(), &[FeedChangedEvent::TYPE]);

    let mut tx = db.begin().await?;
    let row = sqlx::query(
        r#"
        SELECT meta_json
        FROM feed_snapshot
        "#,
    )
    .fetch_one(&mut *tx.tx)
    .await?;
    let meta_json = row.try_get::<String, _>("meta_json")?;
    tx.commit().await?;
    assert!(meta_json.contains("changed title"));
    Ok(())
}
