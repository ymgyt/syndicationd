use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn entry_projection_records_discovered_already_seen_and_changed() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let feed_url = feed_url("entry-projection");

    let first = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("first feed", "first entry", "entry-1"),
        0,
    )
    .await?;
    let first_feed_event = FeedDiscoveredEvent::new(first.feed_url.clone(), first.job_id.clone());
    let recorded = project_feed(&db, first).await?;
    assert_eq!(recorded.types(), &[FeedDiscoveredEvent::TYPE]);
    let recorded = project_entries(&db, EntryProjInput::from(first_feed_event)).await?;
    assert_eq!(recorded.types(), &[EntryDiscoveredEvent::TYPE]);
    let (_, first_source_result_pk) = entry_current_row(&db).await?;

    let second = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("second feed", "first entry", "entry-1"),
        1,
    )
    .await?;
    let second_feed_event = FeedChangedEvent::new(second.feed_url.clone(), second.job_id.clone());
    let recorded = project_feed(&db, second).await?;
    assert_eq!(recorded.types(), &[FeedChangedEvent::TYPE]);
    let recorded = project_entries(&db, EntryProjInput::from(second_feed_event)).await?;
    assert!(recorded.is_empty());
    let (_, second_source_result_pk) = entry_current_row(&db).await?;
    assert_ne!(first_source_result_pk, second_source_result_pk);

    let third = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("third feed", "changed entry", "entry-1"),
        2,
    )
    .await?;
    let third_feed_event = FeedChangedEvent::new(third.feed_url.clone(), third.job_id.clone());
    let recorded = project_feed(&db, third).await?;
    assert_eq!(recorded.types(), &[FeedChangedEvent::TYPE]);
    let recorded = project_entries(&db, EntryProjInput::from(third_feed_event)).await?;
    assert_eq!(recorded.types(), &[EntryChangedEvent::TYPE]);

    let (content_json, third_source_result_pk) = entry_current_row(&db).await?;
    assert!(content_json.contains("changed entry"));
    assert_ne!(second_source_result_pk, third_source_result_pk);
    Ok(())
}
