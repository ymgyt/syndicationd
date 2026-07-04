use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn crawl_completion_records_result_state_and_finished_event() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let feed_url = feed_url("crawl-completion");
    let started_at = Utc.with_ymd_and_hms(2026, 6, 7, 12, 1, 0).unwrap();
    let finished_at = Utc.with_ymd_and_hms(2026, 6, 7, 12, 2, 0).unwrap();

    let mut tx = db.begin().await?;
    store_feed_endpoint(&mut tx, &feed_url, started_at).await?;
    let job = CrawlJob::new(
        CrawlJobId::generate(),
        feed_url.clone(),
        CrawlJobState::Running,
        CrawlJobTrigger::TargetChanged,
        CrawlJobQueueLane::Default,
        0,
        started_at,
        started_at,
        started_at,
    );
    let (_record, events) = CrawlCompletionRecorder::new(&mut tx)
        .record(
            job,
            FeedFetchOutcome::FetchFailed(FeedFetchFailure {
                kind: FeedFetchFailureKind::Timeout,
                message: "deadline exceeded".to_owned(),
            }),
            None,
            finished_at,
        )
        .await?;
    let completion_events = record_generated_events(&mut tx, events).await?;
    tx.commit().await?;

    assert_eq!(completion_events.types(), &[CrawlJobFinishedEvent::TYPE]);

    let mut tx = db.begin().await?;
    let state = tx
        .load_crawl_state(&feed_url)
        .await?
        .expect("crawl state should be recorded");
    assert_eq!(state.last.http_status, None);
    assert_eq!(state.health.failure_streak.value(), 1);
    assert_eq!(
        state.last.error.map(|error| error.kind),
        Some(CrawlStateErrorKind::Fetch(FeedFetchFailureKind::Timeout))
    );
    tx.commit().await?;
    Ok(())
}
