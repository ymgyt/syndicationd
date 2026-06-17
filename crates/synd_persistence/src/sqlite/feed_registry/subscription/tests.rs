use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn uncommitted_subscription_is_rolled_back() -> Result<(), RegistryDbError> {
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
async fn feed_subscription_reads_are_backed_by_feed_endpoint() -> anyhow::Result<()> {
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
    let endpoint_subscriptions = tx
        .load_endpoint_subscriptions(&subscription.feed_url)
        .await?;

    assert_eq!(page.subscriptions, vec![subscription.clone()]);
    assert_eq!(endpoint_subscriptions.feed_url, subscription.feed_url);
    assert_eq!(endpoint_subscriptions.subscriptions.len(), 1);
    assert_eq!(
        endpoint_subscriptions.subscriptions[0].subscription,
        subscription_key(&subscription)
    );
    assert_eq!(
        endpoint_subscriptions.subscriptions[0].crawl_policy,
        subscription.crawl_policy
    );
    Ok(())
}
