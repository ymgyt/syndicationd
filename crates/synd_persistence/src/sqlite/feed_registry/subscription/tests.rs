use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn uncommitted_subscription_is_rolled_back() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    {
        let mut tx = db.begin().await?;
        store_subscription(&mut tx, subscription("rollback")).await?;
    }

    let mut tx = db.begin().await?;
    let page = tx
        .list_subscriptions(SubscriptionsQuery {
            subscriber_id: subscriber_id(),
            after: None,
            first: 10,
        })
        .await?;

    assert!(page.subscriptions.is_empty());
    Ok(())
}

#[tokio::test]
async fn subscription_reads_share_the_feed_ledger() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("shared-read");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    let mut tx = db.begin().await?;
    let page = tx
        .list_subscriptions(SubscriptionsQuery {
            subscriber_id: subscription.subscriber_id.clone(),
            after: None,
            first: 10,
        })
        .await?;
    let feed_subscriptions = tx.load_feed_subscriptions(&subscription.feed_url).await?;

    assert_eq!(page.subscriptions, vec![subscription.clone()]);
    assert_eq!(feed_subscriptions.feed_url, subscription.feed_url);
    assert_eq!(feed_subscriptions.subscriptions.len(), 1);
    assert_eq!(
        feed_subscriptions.subscriptions[0].subscription,
        subscription_key(&subscription)
    );
    assert_eq!(
        feed_subscriptions.subscriptions[0].crawl_policy,
        subscription.crawl_policy
    );
    Ok(())
}

#[tokio::test]
async fn editing_attributes_keeps_subscribed_at() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let mut subscription = subscription("edit-keeps-subscribed-at");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    let subscribed_at = subscription.subscribed_at;
    subscription.crawl_policy = CrawlPolicy::interval(interval(600));
    subscription.subscribed_at = subscribed_at + chrono::Duration::days(1);
    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    let mut tx = db.begin().await?;
    let page = tx
        .list_subscriptions(SubscriptionsQuery {
            subscriber_id: subscription.subscriber_id.clone(),
            after: None,
            first: 10,
        })
        .await?;
    tx.commit().await?;

    assert_eq!(page.subscriptions.len(), 1);
    assert_eq!(
        page.subscriptions[0].crawl_policy,
        subscription.crawl_policy
    );
    assert_eq!(page.subscriptions[0].subscribed_at, subscribed_at);
    Ok(())
}
