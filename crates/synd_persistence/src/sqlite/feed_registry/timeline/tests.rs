use synd_feed::types::Text;

use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn subscription_catchup_reads_complete_entries_from_current_membership() -> anyhow::Result<()>
{
    let db = migrated_db().await?;
    let subscription = subscription("timeline-catchup");
    let first = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry("timeline feed", "old entry", "entry-1"),
        0,
    )
    .await?;
    project_feed(&db, first).await?;
    let second = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry("timeline feed", "current entry", "entry-2"),
        1,
    )
    .await?;
    project_feed(&db, second).await?;
    store_subscription_in_db(&db, subscription.clone()).await?;

    let recorded = project_timeline(
        &db,
        TimelineProjInput::FeedSubscribed(feed_subscribed_event(&subscription)),
    )
    .await?;

    assert_eq!(recorded.types(), &[TimelineChangedEvent::TYPE]);
    let page = list_timeline_entries(&db, subscription.subscriber_id).await?;
    assert_eq!(page.nodes.len(), 1);
    assert_eq!(
        page.nodes[0].entry.title().map(Text::content),
        Some("current entry")
    );
    assert_eq!(
        page.nodes[0].feed_meta.feed.title().map(Text::content),
        Some("timeline feed")
    );
    Ok(())
}
