use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn load_cursor_returns_initial_cursor_for_new_processor() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let mut tx = db.begin().await?;

    let cursor = tx.load_cursor(ProcessorId::CrawlTargetProjection).await?;
    tx.commit().await?;

    assert_eq!(
        cursor,
        EventCursor::initial(ProcessorId::CrawlTargetProjection)
    );
    Ok(())
}

#[tokio::test]
async fn append_and_read_subscription_events_for_processor() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    {
        let mut tx = db.begin().await?;
        let mut recorded = RecordedEvents::with_capacity(2);
        let clock = TestClock(test_occurred_at());
        EventRecorder::new(&mut tx, &mut recorded, &clock)
            .record_all([subscribed_event("subscribed"), changed_event("changed")])
            .await?;
        tx.commit().await?;
    }

    let mut tx = db.begin().await?;
    let cursor = tx.load_cursor(ProcessorId::CrawlTargetProjection).await?;
    let batch = tx
        .read_after(&cursor, subscription_lifecycle_interests())
        .await?;
    tx.commit().await?;

    assert_eq!(batch.events().len(), 2);
    assert_eq!(batch.events()[0].event(), &subscribed_event("subscribed"));
    assert_eq!(batch.events()[0].occurred_at(), test_occurred_at());
    assert_eq!(batch.events()[1].event(), &changed_event("changed"));
    assert_eq!(batch.events()[1].occurred_at(), test_occurred_at());
    assert_eq!(
        batch.scanned_cursor(),
        &EventCursor::at(
            ProcessorId::CrawlTargetProjection,
            EventCursorPos::position("2")
        )
    );
    Ok(())
}
