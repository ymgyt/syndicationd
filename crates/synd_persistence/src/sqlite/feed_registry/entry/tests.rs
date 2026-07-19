use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn entry_projection_records_discovered_unchanged_and_changed() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let feed_url = feed_url("entry-projection");

    let first = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("first feed", "first entry", "entry-1"),
        0,
    )
    .await?;
    let first_input = entry_proj_input(&first);
    let recorded = project_feed(&db, first).await?;
    assert_eq!(recorded.types(), &[FeedDiscoveredEvent::TYPE]);
    let recorded = project_entries(&db, first_input).await?;
    assert_eq!(recorded.types(), &[EntryDiscoveredEvent::TYPE]);

    // Same entry content in a changed feed body: no entry change, no write.
    let second = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("second feed", "first entry", "entry-1"),
        1,
    )
    .await?;
    let second_input = entry_proj_input(&second);
    let recorded = project_feed(&db, second).await?;
    assert_eq!(recorded.types(), &[FeedChangedEvent::TYPE]);
    let recorded = project_entries(&db, second_input).await?;
    assert!(recorded.is_empty());
    let attrs_json = entry_current_row(&db).await?;
    assert!(attrs_json.contains("first entry"));

    let third = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("third feed", "changed entry", "entry-1"),
        2,
    )
    .await?;
    let third_input = entry_proj_input(&third);
    let recorded = project_feed(&db, third).await?;
    assert_eq!(recorded.types(), &[FeedChangedEvent::TYPE]);
    let recorded = project_entries(&db, third_input).await?;
    assert_eq!(recorded.types(), &[EntryChangedEvent::TYPE]);

    let attrs_json = entry_current_row(&db).await?;
    assert!(attrs_json.contains("changed entry"));
    Ok(())
}
