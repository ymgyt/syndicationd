use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn timeline_projection_catches_up_existing_feed_entries_after_subscription()
-> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("timeline-projection");

    let crawl = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry("timeline feed", "timeline entry", "entry-1"),
        0,
    )
    .await?;
    let feed_event = FeedDiscoveredEvent::new(crawl.feed_url.clone(), crawl.job_id.clone());
    let recorded = project_feed(&db, crawl).await?;
    assert_eq!(recorded.types(), &[FeedDiscoveredEvent::TYPE]);
    let recorded = project_entries(&db, EntryProjInput::from(feed_event)).await?;
    assert_eq!(recorded.types(), &[EntryDiscoveredEvent::TYPE]);

    store_subscription_in_db(&db, subscription.clone()).await?;

    let subscribed = feed_subscribed_event(&subscription);
    let recorded = project_timeline(&db, timeline_feed_subscribed(subscribed)).await?;
    assert_eq!(recorded.types(), &[TimelineChangedEvent::TYPE]);

    let mut tx = db.begin().await?;
    let page = tx
        .list_timeline_items(TimelineItemsQuery {
            subscriber_id: subscription.subscriber_id.clone(),
            feed_url: None,
            after: None,
            first: 10,
        })
        .await?;
    tx.commit().await?;

    assert_eq!(page.nodes.len(), 1);
    assert!(!page.has_next_page);
    let node = &page.nodes[0];
    assert_eq!(node.subscription, subscription);
    assert_eq!(node.attrs.title.as_deref(), Some("timeline entry"));
    assert_eq!(node.feed_meta.feed.url(), &feed_url("timeline-projection"));
    Ok(())
}

#[tokio::test]
async fn list_timeline_items_can_filter_by_feed_url() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let first = subscription("timeline-filter-one");
    let second = subscription("timeline-filter-two");

    let first_crawl = record_fetched_crawl(
        &db,
        &first.feed_url,
        rss_body_with_entry("first feed", "first entry", "entry-1"),
        0,
    )
    .await?;
    let second_crawl = record_fetched_crawl(
        &db,
        &second.feed_url,
        rss_body_with_entry("second feed", "second entry", "entry-2"),
        1,
    )
    .await?;

    let first_feed_event =
        FeedDiscoveredEvent::new(first_crawl.feed_url.clone(), first_crawl.job_id.clone());
    let second_feed_event =
        FeedDiscoveredEvent::new(second_crawl.feed_url.clone(), second_crawl.job_id.clone());
    let _ = project_feed(&db, first_crawl).await?;
    let _ = project_entries(&db, EntryProjInput::from(first_feed_event)).await?;
    let _ = project_feed(&db, second_crawl).await?;
    let _ = project_entries(&db, EntryProjInput::from(second_feed_event)).await?;

    store_subscription_in_db(&db, first.clone()).await?;
    store_subscription_in_db(&db, second.clone()).await?;
    let _ = project_timeline_batch(
        &db,
        vec![
            timeline_feed_subscribed(feed_subscribed_event(&first)),
            timeline_feed_subscribed(feed_subscribed_event(&second)),
        ],
    )
    .await?;

    let mut tx = db.begin().await?;
    let page = tx
        .list_timeline_items(TimelineItemsQuery {
            subscriber_id: first.subscriber_id.clone(),
            feed_url: Some(first.feed_url.clone()),
            after: None,
            first: 10,
        })
        .await?;
    tx.commit().await?;

    assert_eq!(page.nodes.len(), 1);
    assert_eq!(page.nodes[0].subscription.feed_url, first.feed_url);
    assert_eq!(page.nodes[0].attrs.title.as_deref(), Some("first entry"));
    Ok(())
}

#[tokio::test]
async fn timeline_projection_does_not_emit_changed_when_catchup_inserts_nothing()
-> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("timeline-idempotent");

    let crawl = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry("timeline feed", "timeline entry", "entry-1"),
        0,
    )
    .await?;
    let feed_event = FeedDiscoveredEvent::new(crawl.feed_url.clone(), crawl.job_id.clone());
    let _ = project_feed(&db, crawl).await?;
    let _ = project_entries(&db, EntryProjInput::from(feed_event)).await?;

    store_subscription_in_db(&db, subscription.clone()).await?;

    let subscribed = feed_subscribed_event(&subscription);
    let _ = project_timeline(&db, timeline_feed_subscribed(subscribed.clone())).await?;
    let recorded = project_timeline(&db, timeline_feed_subscribed(subscribed)).await?;

    assert!(recorded.is_empty());
    Ok(())
}

#[tokio::test]
async fn timeline_projection_preserves_feed_lifecycle_order_inside_batch() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("timeline-lifecycle-order");

    let crawl = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry("timeline feed", "timeline entry", "entry-1"),
        0,
    )
    .await?;
    let feed_event = FeedDiscoveredEvent::new(crawl.feed_url.clone(), crawl.job_id.clone());
    let _ = project_feed(&db, crawl).await?;
    let _ = project_entries(&db, EntryProjInput::from(feed_event)).await?;

    store_subscription_in_db(&db, subscription.clone()).await?;

    let key = subscription_key(&subscription);
    let recorded = project_timeline_batch(
        &db,
        vec![
            timeline_feed_subscribed(FeedSubscribedEvent::new(
                key.clone(),
                subscription_attrs(&subscription),
            )),
            timeline_feed_unsubscribed(FeedUnsubscribedEvent::new(key.clone())),
            timeline_feed_subscribed(FeedSubscribedEvent::new(
                key,
                subscription_attrs(&subscription),
            )),
        ],
    )
    .await?;
    assert_eq!(recorded.types(), &[TimelineChangedEvent::TYPE]);

    let page = list_timeline_items(&db, subscription.subscriber_id.clone()).await?;
    assert_eq!(page.nodes.len(), 1);
    assert_eq!(page.nodes[0].attrs.title.as_deref(), Some("timeline entry"));
    Ok(())
}

#[tokio::test]
async fn timeline_projection_adds_discovered_entry_for_active_subscription() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("timeline-entry-discovered");

    let crawl = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry("timeline feed", "timeline entry", "entry-1"),
        0,
    )
    .await?;
    let feed_event = FeedDiscoveredEvent::new(crawl.feed_url.clone(), crawl.job_id.clone());
    let _ = project_feed(&db, crawl.clone()).await?;
    let _ = project_entries(&db, EntryProjInput::from(feed_event)).await?;
    let entry_id = current_entry_id(&db).await?;

    store_subscription_in_db(&db, subscription.clone()).await?;

    let recorded = project_timeline(
        &db,
        timeline_entry_discovered(EntryDiscoveredEvent::new(
            subscription.feed_url.clone(),
            entry_id,
            crawl.job_id,
        )),
    )
    .await?;
    assert_eq!(recorded.types(), &[TimelineChangedEvent::TYPE]);

    let page = list_timeline_items(&db, subscription.subscriber_id.clone()).await?;
    assert_eq!(page.nodes.len(), 1);
    assert_eq!(page.nodes[0].attrs.title.as_deref(), Some("timeline entry"));
    Ok(())
}

#[tokio::test]
async fn timeline_projection_ignores_discovered_entry_without_subscription() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let feed_url = feed_url("timeline-entry-without-subscription");

    let crawl = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("timeline feed", "timeline entry", "entry-1"),
        0,
    )
    .await?;
    let feed_event = FeedDiscoveredEvent::new(crawl.feed_url.clone(), crawl.job_id.clone());
    let _ = project_feed(&db, crawl.clone()).await?;
    let _ = project_entries(&db, EntryProjInput::from(feed_event)).await?;
    let entry_id = current_entry_id(&db).await?;

    let recorded = project_timeline(
        &db,
        timeline_entry_discovered(EntryDiscoveredEvent::new(feed_url, entry_id, crawl.job_id)),
    )
    .await?;

    assert!(recorded.is_empty());
    Ok(())
}

#[tokio::test]
async fn timeline_projection_invalidates_changed_entry_payload() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("timeline-entry-payload");

    let first = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry("timeline feed", "old title", "entry-1"),
        0,
    )
    .await?;
    let first_feed_event = FeedDiscoveredEvent::new(first.feed_url.clone(), first.job_id.clone());
    let _ = project_feed(&db, first.clone()).await?;
    let _ = project_entries(&db, EntryProjInput::from(first_feed_event)).await?;
    let entry_id = current_entry_id(&db).await?;

    store_subscription_in_db(&db, subscription.clone()).await?;
    let _ = project_timeline(
        &db,
        timeline_entry_discovered(EntryDiscoveredEvent::new(
            subscription.feed_url.clone(),
            entry_id.clone(),
            first.job_id,
        )),
    )
    .await?;

    let second = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry("timeline feed", "new title", "entry-1"),
        1,
    )
    .await?;
    let second_feed_event = FeedChangedEvent::new(second.feed_url.clone(), second.job_id.clone());
    let _ = project_feed(&db, second.clone()).await?;
    let recorded = project_entries(&db, EntryProjInput::from(second_feed_event)).await?;
    assert_eq!(recorded.types(), &[EntryChangedEvent::TYPE]);

    let recorded = project_timeline(
        &db,
        timeline_entry_changed(EntryChangedEvent::new(
            subscription.feed_url.clone(),
            entry_id,
            second.job_id,
        )),
    )
    .await?;
    assert_eq!(recorded.types(), &[TimelineChangedEvent::TYPE]);

    let page = list_timeline_items(&db, subscription.subscriber_id.clone()).await?;
    assert_eq!(page.nodes.len(), 1);
    assert_eq!(page.nodes[0].attrs.title.as_deref(), Some("new title"));
    Ok(())
}

#[tokio::test]
async fn timeline_projection_updates_changed_entry_order() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("timeline-entry-order");
    let older = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
    let newer = Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap();

    let first = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry_published_at("timeline feed", "old title", "entry-1", older),
        0,
    )
    .await?;
    let first_feed_event = FeedDiscoveredEvent::new(first.feed_url.clone(), first.job_id.clone());
    let _ = project_feed(&db, first.clone()).await?;
    let _ = project_entries(&db, EntryProjInput::from(first_feed_event)).await?;
    let entry_id = current_entry_id(&db).await?;

    store_subscription_in_db(&db, subscription.clone()).await?;
    let _ = project_timeline(
        &db,
        timeline_entry_discovered(EntryDiscoveredEvent::new(
            subscription.feed_url.clone(),
            entry_id.clone(),
            first.job_id,
        )),
    )
    .await?;
    let page = list_timeline_items(&db, subscription.subscriber_id.clone()).await?;
    assert_eq!(page.nodes[0].cursor.order_time(), older);

    let second = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry_published_at("timeline feed", "new title", "entry-1", newer),
        1,
    )
    .await?;
    let second_feed_event = FeedChangedEvent::new(second.feed_url.clone(), second.job_id.clone());
    let _ = project_feed(&db, second.clone()).await?;
    let _ = project_entries(&db, EntryProjInput::from(second_feed_event)).await?;

    let recorded = project_timeline(
        &db,
        timeline_entry_changed(EntryChangedEvent::new(
            subscription.feed_url.clone(),
            entry_id,
            second.job_id,
        )),
    )
    .await?;
    assert_eq!(recorded.types(), &[TimelineChangedEvent::TYPE]);

    let page = list_timeline_items(&db, subscription.subscriber_id.clone()).await?;
    assert_eq!(page.nodes[0].cursor.order_time(), newer);
    Ok(())
}

#[tokio::test]
async fn timeline_projection_coalesces_entry_invalidations_by_feed() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("timeline-entry-coalesce");

    let crawl = record_fetched_crawl(
        &db,
        &subscription.feed_url,
        rss_body_with_entry("timeline feed", "timeline entry", "entry-1"),
        0,
    )
    .await?;
    let feed_event = FeedDiscoveredEvent::new(crawl.feed_url.clone(), crawl.job_id.clone());
    let _ = project_feed(&db, crawl.clone()).await?;
    let _ = project_entries(&db, EntryProjInput::from(feed_event)).await?;
    let entry_id = current_entry_id(&db).await?;

    store_subscription_in_db(&db, subscription.clone()).await?;

    let input = timeline_entry_discovered(EntryDiscoveredEvent::new(
        subscription.feed_url.clone(),
        entry_id,
        crawl.job_id,
    ));
    let recorded = project_timeline_batch(&db, vec![input.clone(), input]).await?;
    assert_eq!(recorded.types(), &[TimelineChangedEvent::TYPE]);

    let events = read_timeline_events(&db).await?;
    let Some(Event::TimelineChanged(event)) = events.last() else {
        anyhow::bail!("expected timeline changed event");
    };
    assert_eq!(event.affected_feeds, vec![subscription.feed_url]);
    Ok(())
}
