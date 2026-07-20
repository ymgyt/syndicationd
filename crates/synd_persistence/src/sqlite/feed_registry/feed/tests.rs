use synd_feed::types::Text;

use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn feed_projection_stores_a_complete_current_feed() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let feed_url = feed_url("feed-projection");
    let crawl = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("feed title", "entry title", "entry-1"),
        0,
    )
    .await?;

    let recorded = project_feed(&db, crawl).await?;

    assert_eq!(recorded.types(), &[EntryDiscoveredEvent::TYPE]);
    let mut tx = db.begin().await?;
    let mut feeds = tx.load_feeds(std::slice::from_ref(&feed_url)).await?;
    tx.commit().await?;
    let feed = feeds.remove(&feed_url).unwrap();
    assert_eq!(feed.meta().title().map(Text::content), Some("feed title"));
    assert_eq!(
        feed.entries()
            .next()
            .and_then(|entry| entry.title())
            .map(Text::content),
        Some("entry title")
    );
    Ok(())
}

#[tokio::test]
async fn feed_projection_replaces_membership_without_deleting_catalog_history() -> anyhow::Result<()>
{
    let db = migrated_db().await?;
    let feed_url = feed_url("feed-membership");
    let first = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("feed", "old entry", "entry-1"),
        0,
    )
    .await?;
    project_feed(&db, first).await?;
    let second = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("feed", "current entry", "entry-2"),
        1,
    )
    .await?;

    project_feed(&db, second).await?;

    let mut tx = db.begin().await?;
    let mut feeds = tx.load_feeds(std::slice::from_ref(&feed_url)).await?;
    let catalog_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM entry")
        .fetch_one(&mut *tx.tx)
        .await?;
    tx.commit().await?;
    let feed = feeds.remove(&feed_url).unwrap();
    let titles = feed
        .entries()
        .filter_map(|entry| entry.title().map(Text::content))
        .collect::<Vec<_>>();
    assert_eq!(titles, ["current entry"]);
    assert_eq!(catalog_count, 2);
    Ok(())
}
