use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn crawl_target_projection_activates_target_after_subscription() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("crawl-target-subscribed");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    project_crawl_targets(
        &db,
        vec![SubEvent::Subscribed(feed_subscribed_event(&subscription))],
    )
    .await?;

    let mut tx = db.begin().await?;
    let target = tx
        .load_target(&subscription.feed_url)
        .await?
        .expect("crawl target should be projected");

    let CrawlTargetState::Active { effective_policy } = target.state else {
        anyhow::bail!("crawl target should be active");
    };
    assert_eq!(
        effective_policy.polling,
        PollingPolicy::Interval {
            interval: interval(3600)
        }
    );
    Ok(())
}

#[tokio::test]
async fn crawl_target_projection_emits_target_activated_event() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("crawl-target-activated-event");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    project_crawl_targets(
        &db,
        vec![SubEvent::Subscribed(feed_subscribed_event(&subscription))],
    )
    .await?;

    let events = read_crawl_target_events(&db).await?;
    assert_eq!(events.len(), 1);
    let Event::CrawlTargetActivated(event) = &events[0] else {
        anyhow::bail!("unexpected crawl event: {:?}", events[0]);
    };
    assert_eq!(event.feed_url, subscription.feed_url);
    assert_eq!(event.policy, subscription.crawl_policy);
    Ok(())
}

#[tokio::test]
async fn crawl_target_projection_aggregates_multiple_subscriptions() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let one_hour = subscription_with(
        SubscriberId::new("one-hour"),
        "crawl-target-aggregate",
        CrawlPolicy::interval(interval(3600)),
    );
    let ten_minutes = subscription_with(
        SubscriberId::new("ten-minutes"),
        "crawl-target-aggregate",
        CrawlPolicy::interval(interval(600)),
    );
    let manual = subscription_with(
        SubscriberId::new("manual"),
        "crawl-target-aggregate",
        CrawlPolicy::manual(),
    );

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, one_hour.clone()).await?;
    store_subscription(&mut tx, ten_minutes).await?;
    store_subscription(&mut tx, manual).await?;
    tx.commit().await?;

    project_crawl_targets(
        &db,
        vec![SubEvent::Subscribed(feed_subscribed_event(&one_hour))],
    )
    .await?;

    let mut tx = db.begin().await?;
    let target = tx
        .load_target(&one_hour.feed_url)
        .await?
        .expect("crawl target should be projected");

    let CrawlTargetState::Active { effective_policy } = target.state else {
        anyhow::bail!("crawl target should be active");
    };
    assert_eq!(
        effective_policy.polling,
        PollingPolicy::Interval {
            interval: interval(600)
        }
    );
    Ok(())
}

#[tokio::test]
async fn crawl_target_projection_recalculates_after_subscription_change() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let mut subscription = subscription("crawl-target-changed");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    project_crawl_targets(
        &db,
        vec![SubEvent::Subscribed(feed_subscribed_event(&subscription))],
    )
    .await?;

    subscription.crawl_policy = CrawlPolicy::interval(interval(300));
    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    project_crawl_targets(
        &db,
        vec![SubEvent::Changed(subscription_changed_event(&subscription))],
    )
    .await?;

    let mut tx = db.begin().await?;
    let target = tx
        .load_target(&subscription.feed_url)
        .await?
        .expect("crawl target should be projected");

    let CrawlTargetState::Active { effective_policy } = target.state else {
        anyhow::bail!("crawl target should be active");
    };
    assert_eq!(
        effective_policy.polling,
        PollingPolicy::Interval {
            interval: interval(300)
        }
    );
    Ok(())
}

#[tokio::test]
async fn crawl_target_projection_emits_target_policy_changed_event() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let mut subscription = subscription("crawl-target-policy-changed-event");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    project_crawl_targets(
        &db,
        vec![SubEvent::Subscribed(feed_subscribed_event(&subscription))],
    )
    .await?;

    subscription.crawl_policy = CrawlPolicy::interval(interval(300));
    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    project_crawl_targets(
        &db,
        vec![SubEvent::Changed(subscription_changed_event(&subscription))],
    )
    .await?;

    let events = read_crawl_target_events(&db).await?;
    assert_eq!(events.len(), 2);
    let Event::CrawlTargetPolicyChanged(event) = &events[1] else {
        anyhow::bail!("unexpected crawl event: {:?}", events[1]);
    };
    assert_eq!(event.feed_url, subscription.feed_url);
    assert_eq!(event.policy, subscription.crawl_policy);
    Ok(())
}

#[tokio::test]
async fn crawl_target_projection_inactivates_target_after_last_unsubscribe() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("crawl-target-unsubscribed");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    project_crawl_targets(
        &db,
        vec![SubEvent::Subscribed(feed_subscribed_event(&subscription))],
    )
    .await?;

    let mut tx = db.begin().await?;
    tx.delete_subscription(&subscription.subscriber_id, &subscription.feed_url)
        .await?;
    tx.commit().await?;

    project_crawl_targets(
        &db,
        vec![SubEvent::Unsubscribed(FeedUnsubscribedEvent::new(
            subscription_key(&subscription),
        ))],
    )
    .await?;

    let mut tx = db.begin().await?;
    let target = tx
        .load_target(&subscription.feed_url)
        .await?
        .expect("crawl target should be projected");

    assert_eq!(target.state, CrawlTargetState::Inactive);
    Ok(())
}

#[tokio::test]
async fn crawl_target_projection_emits_target_deactivated_event() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("crawl-target-deactivated-event");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    project_crawl_targets(
        &db,
        vec![SubEvent::Subscribed(feed_subscribed_event(&subscription))],
    )
    .await?;

    let mut tx = db.begin().await?;
    tx.delete_subscription(&subscription.subscriber_id, &subscription.feed_url)
        .await?;
    tx.commit().await?;

    project_crawl_targets(
        &db,
        vec![SubEvent::Unsubscribed(FeedUnsubscribedEvent::new(
            subscription_key(&subscription),
        ))],
    )
    .await?;

    let events = read_crawl_target_events(&db).await?;
    assert_eq!(events.len(), 2);
    let Event::CrawlTargetDeactivated(event) = &events[1] else {
        anyhow::bail!("unexpected crawl event: {:?}", events[1]);
    };
    assert_eq!(event.feed_url, subscription.feed_url);
    Ok(())
}

#[tokio::test]
async fn manual_request_is_set_once_and_cleared_by_a_later_crawl() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let subscription = subscription("crawl-target-manual");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;
    project_crawl_targets(
        &db,
        vec![SubEvent::Subscribed(feed_subscribed_event(&subscription))],
    )
    .await?;

    let requested_at = test_occurred_at();
    let mut tx = db.begin().await?;
    tx.set_manual_request(&subscription.feed_url, requested_at)
        .await?;
    // A second request while one is pending keeps the earlier instant.
    tx.set_manual_request(
        &subscription.feed_url,
        requested_at + chrono::Duration::minutes(5),
    )
    .await?;
    let input = tx
        .load_crawl_due_input(&subscription.feed_url)
        .await?
        .expect("active target should have a due input");
    assert_eq!(input.manual_requested_at, Some(requested_at));

    // A crawl that started before the request does not serve it.
    tx.clear_manual_request(
        &subscription.feed_url,
        requested_at - chrono::Duration::minutes(1),
    )
    .await?;
    let input = tx
        .load_crawl_due_input(&subscription.feed_url)
        .await?
        .expect("active target should have a due input");
    assert_eq!(input.manual_requested_at, Some(requested_at));

    // A crawl that started at or after the request serves and clears it.
    tx.clear_manual_request(&subscription.feed_url, requested_at)
        .await?;
    let input = tx
        .load_crawl_due_input(&subscription.feed_url)
        .await?
        .expect("active target should have a due input");
    assert_eq!(input.manual_requested_at, None);
    tx.commit().await?;
    Ok(())
}

#[tokio::test]
async fn due_inputs_list_only_active_targets_with_their_state() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let active = subscription("crawl-due-active");
    let inactive = subscription("crawl-due-inactive");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, active.clone()).await?;
    store_subscription(&mut tx, inactive.clone()).await?;
    tx.commit().await?;
    project_crawl_targets(
        &db,
        vec![
            SubEvent::Subscribed(feed_subscribed_event(&active)),
            SubEvent::Subscribed(feed_subscribed_event(&inactive)),
        ],
    )
    .await?;

    let mut tx = db.begin().await?;
    tx.delete_subscription(&inactive.subscriber_id, &inactive.feed_url)
        .await?;
    tx.commit().await?;
    project_crawl_targets(
        &db,
        vec![SubEvent::Unsubscribed(FeedUnsubscribedEvent::new(
            subscription_key(&inactive),
        ))],
    )
    .await?;

    let mut tx = db.begin().await?;
    let inputs = tx.list_crawl_due_inputs().await?;
    tx.commit().await?;

    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].feed_url, active.feed_url);
    assert_eq!(
        inputs[0].polling,
        PollingPolicy::Interval {
            interval: interval(3600)
        }
    );
    assert_eq!(inputs[0].manual_requested_at, None);
    // Never crawled: no state joined.
    assert!(inputs[0].state.is_none());
    Ok(())
}
