use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn crawl_schedule_candidates_and_job_enqueue_are_persisted() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let now = Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();
    let subscription = subscription("crawl-schedule-candidate");

    let mut tx = db.begin().await?;
    store_subscription(&mut tx, subscription.clone()).await?;
    tx.commit().await?;

    project_crawl_targets(
        &db,
        vec![SubscriptionLifecycle::Subscribed(FeedSubscribedEvent::new(
            subscription_key(&subscription),
        ))],
    )
    .await?;

    let mut tx = db.begin().await?;
    let candidates = tx.list_candidates(now, 10).await?;
    assert_eq!(candidates.len(), 1);
    let candidate = &candidates[0];
    assert_eq!(candidate.target.feed_url, subscription.feed_url);
    assert!(candidate.schedule.is_none());
    assert!(candidate.active_job.is_none());

    tx.upsert_schedule(UpsertCrawlScheduleCommand::new(
        subscription.feed_url.clone(),
        candidate.target.target_updated_at,
        Some(now),
        now,
    ))
    .await?;

    let result = tx
        .enqueue_job(EnqueueCrawlJobCommand::new(
            subscription.feed_url.clone(),
            CrawlJobTrigger::TargetChanged,
            CrawlJobQueueLane::Default,
            0,
            now,
            now,
        ))
        .await?;
    let EnqueueCrawlJobOutcome::Enqueued(job) = result else {
        anyhow::bail!("expected job to be enqueued");
    };
    assert_eq!(job.feed_url, subscription.feed_url);
    assert_eq!(job.trigger, CrawlJobTrigger::TargetChanged);

    let result = tx
        .enqueue_job(EnqueueCrawlJobCommand::new(
            subscription.feed_url.clone(),
            CrawlJobTrigger::TargetChanged,
            CrawlJobQueueLane::Default,
            0,
            now,
            now,
        ))
        .await?;
    assert_eq!(result, EnqueueCrawlJobOutcome::AlreadyActive);

    let result = tx
        .claim_job(ClaimCrawlJobCommand::new(CrawlJobQueueLane::Default, now))
        .await?;
    let ClaimCrawlJobOutcome::Claimed(claimed) = result else {
        anyhow::bail!("expected job to be claimed");
    };
    assert_eq!(claimed.feed_url, subscription.feed_url);
    assert_eq!(claimed.state, CrawlJobState::Running);
    assert_eq!(claimed.queue, CrawlJobQueueLane::Default);
    tx.commit().await?;

    let mut tx = db.begin().await?;
    let candidates = tx.list_candidates(now, 10).await?;
    assert_eq!(candidates.len(), 1);
    assert!(candidates[0].schedule.is_some());
    assert!(candidates[0].active_job.is_some());
    Ok(())
}
