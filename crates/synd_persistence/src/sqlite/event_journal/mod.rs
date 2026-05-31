use sqlx::{QueryBuilder, Row, Sqlite};
use synd_registry::event::{
    Event, EventCursor, EventCursorPos, EventEncoding, EventInterests, EventJournal,
    EventJournalError, EventJournalResult, EventReadBatch, JournaledEvent, ProcessorId,
};

use super::SqliteDatabase;

/// SQLite-backed event journal for registry events.
#[derive(Clone)]
pub struct SqliteEventJournal {
    db: SqliteDatabase,
}

impl SqliteEventJournal {
    pub fn new(db: SqliteDatabase) -> Self {
        Self { db }
    }
}

impl EventJournal for SqliteEventJournal {
    async fn append(&self, event: Event) -> EventJournalResult<()> {
        let encoded = event.encode().map_err(map_error)?;
        let mut tx = self.db.begin().await.map_err(map_error)?;

        sqlx::query(
            r"
            INSERT INTO event_journal (event_type, payload_json)
            VALUES (?, ?)
            ",
        )
        .bind(encoded.event_type)
        .bind(encoded.payload_json)
        .execute(&mut *tx)
        .await
        .map_err(map_error)?;

        tx.commit().await.map_err(map_error)?;
        Ok(())
    }

    async fn read_after(
        &self,
        cursor: &EventCursor,
        interests: EventInterests,
    ) -> EventJournalResult<EventReadBatch> {
        let position = decode_position(cursor.position())?;
        let processor = cursor.processor();
        let event_types = interests
            .kinds()
            .iter()
            .copied()
            .map(synd_registry::event::EventKind::event_type)
            .collect::<Vec<_>>();
        let mut tx = self.db.begin().await.map_err(map_error)?;

        let scanned_position = sqlx::query(
            r"
            SELECT COALESCE(MAX(position), ?) AS scanned_position
            FROM event_journal
            WHERE position > ?
            ",
        )
        .bind(position)
        .bind(position)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_error)?
        .try_get::<i64, _>("scanned_position")
        .map_err(map_error)?;

        let scanned_cursor = EventCursor::at(
            processor,
            EventCursorPos::position(scanned_position.to_string()),
        );

        if event_types.is_empty() || scanned_position <= position {
            tx.commit().await.map_err(map_error)?;
            return Ok(EventReadBatch::empty(scanned_cursor));
        }

        let mut query = QueryBuilder::<Sqlite>::new(
            r"
            SELECT position, event_type, payload_json
            FROM event_journal
            WHERE position > ",
        );
        query.push_bind(position);
        query.push(" AND position <= ");
        query.push_bind(scanned_position);
        query.push(" AND event_type IN (");
        let mut separated = query.separated(", ");
        for event_type in event_types {
            separated.push_bind(event_type);
        }
        separated.push_unseparated(") ORDER BY position");

        let rows = query.build().fetch_all(&mut *tx).await.map_err(map_error)?;
        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let position = row.try_get::<i64, _>("position").map_err(map_error)?;
            let event_type = row.try_get::<String, _>("event_type").map_err(map_error)?;
            let payload_json = row
                .try_get::<String, _>("payload_json")
                .map_err(map_error)?;
            events.push(JournaledEvent::new(
                EventCursor::at(processor, EventCursorPos::position(position.to_string())),
                Event::decode(&event_type, &payload_json).map_err(map_error)?,
            ));
        }

        tx.commit().await.map_err(map_error)?;
        Ok(EventReadBatch::new(events, scanned_cursor))
    }

    async fn load_cursor(&self, processor: ProcessorId) -> EventJournalResult<EventCursor> {
        let mut tx = self.db.begin().await.map_err(map_error)?;
        let row = sqlx::query(
            r"
            SELECT position
            FROM event_cursor
            WHERE consumer = ?
            ",
        )
        .bind(processor.as_str())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_error)?;
        tx.commit().await.map_err(map_error)?;

        let Some(row) = row else {
            return Ok(EventCursor::initial(processor));
        };
        let position = row.try_get::<i64, _>("position").map_err(map_error)?;
        Ok(EventCursor::at(
            processor,
            EventCursorPos::position(position.to_string()),
        ))
    }
}

fn decode_position(position: &EventCursorPos) -> EventJournalResult<i64> {
    match position {
        EventCursorPos::Initial => Ok(0),
        EventCursorPos::Position(position) => {
            let position = position.parse::<i64>().map_err(map_error)?;
            if position < 0 {
                return Err(EventJournalError::Internal(anyhow::anyhow!(
                    "event cursor position must be non-negative: {position}"
                )));
            }
            Ok(position)
        }
    }
}

fn map_error(err: impl Into<anyhow::Error>) -> EventJournalError {
    EventJournalError::Internal(err.into())
}

#[cfg(test)]
mod tests {
    use synd_feed::types::FeedUrl;
    use synd_registry::{
        SubscriberId, SubscriptionKey,
        event::{FeedSubscribed, SubEvent, SubEventKind, SubscriptionChanged},
    };

    use super::*;

    async fn migrated_journal() -> anyhow::Result<SqliteEventJournal> {
        let db = SqliteDatabase::in_memory().await?;
        db.migrate().await?;
        Ok(SqliteEventJournal::new(db))
    }

    fn subscribed_event() -> Event {
        Event::Sub(SubEvent::FeedSubscribed(FeedSubscribed::new(subscription(
            "subscribed",
        ))))
    }

    fn changed_event() -> Event {
        Event::Sub(SubEvent::SubscriptionChanged(SubscriptionChanged::new(
            subscription("changed"),
        )))
    }

    fn subscription(path: &str) -> SubscriptionKey {
        SubscriptionKey::new(
            SubscriberId::new("local"),
            FeedUrl::parse(&format!("https://example.com/{path}.xml")).unwrap(),
        )
    }

    fn subscription_lifecycle_interests() -> EventInterests {
        EventInterests::new([
            SubEventKind::FeedSubscribed.into(),
            SubEventKind::SubscriptionChanged.into(),
            SubEventKind::FeedUnsubscribed.into(),
        ])
    }

    #[tokio::test]
    async fn load_cursor_returns_initial_cursor_for_new_processor() -> anyhow::Result<()> {
        let journal = migrated_journal().await?;

        let cursor = journal
            .load_cursor(ProcessorId::CrawlTargetProjection)
            .await?;

        assert_eq!(
            cursor,
            EventCursor::initial(ProcessorId::CrawlTargetProjection)
        );
        Ok(())
    }

    #[tokio::test]
    async fn append_and_read_subscription_events_for_processor() -> anyhow::Result<()> {
        let journal = migrated_journal().await?;
        journal.append(subscribed_event()).await?;
        journal.append(changed_event()).await?;

        let cursor = journal
            .load_cursor(ProcessorId::CrawlTargetProjection)
            .await?;
        let batch = journal
            .read_after(&cursor, subscription_lifecycle_interests())
            .await?;

        assert_eq!(batch.events().len(), 2);
        assert_eq!(batch.events()[0].event(), &subscribed_event());
        assert_eq!(batch.events()[1].event(), &changed_event());
        assert_eq!(
            batch.scanned_cursor(),
            &EventCursor::at(
                ProcessorId::CrawlTargetProjection,
                EventCursorPos::position("2")
            )
        );
        Ok(())
    }
}
