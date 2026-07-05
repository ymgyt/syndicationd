use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Sqlite, Transaction};
use synd_registry::{
    RegistryDbResult,
    event::{
        Event, EventCursor, EventCursorPos, EventEncoding, EventInterests, EventJournal,
        EventJournalAppend, EventReadBatch, EventType, JournaledEvent, ProcessorId,
    },
};

use super::{
    SqliteRegistryTx,
    error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
};

async fn append_event(
    tx: &mut Transaction<'_, Sqlite>,
    event: Event,
    occurred_at: DateTime<Utc>,
) -> SqliteResult<EventType> {
    let encoded = event.encode()?;
    let event_type = encoded.event_type;
    sqlx::query(
        r"
        INSERT INTO event_journal (occurred_at, payload_json)
        VALUES (?, ?)
        ",
    )
    .bind(occurred_at)
    .bind(encoded.payload_json)
    .execute(&mut **tx)
    .await?;

    Ok(event_type)
}

async fn read_after(
    tx: &mut Transaction<'_, Sqlite>,
    cursor: &EventCursor,
    interests: EventInterests,
) -> SqliteResult<EventReadBatch> {
    let position = decode_event_cursor_position(cursor.position())?;
    let processor = cursor.processor();
    let event_types = interests
        .types()
        .iter()
        .copied()
        .map(<EventType as Into<&'static str>>::into)
        .collect::<Vec<_>>();

    let scanned = sqlx::query_as::<_, ScannedPositionRow>(
        r"
        SELECT COALESCE(MAX(position), ?) AS scanned_position
        FROM event_journal
        WHERE position > ?
        ",
    )
    .bind(position)
    .bind(position)
    .fetch_one(&mut **tx)
    .await?;

    let scanned_cursor = EventCursor::at(
        processor,
        EventCursorPos::position(scanned.scanned_position.to_string()),
    );

    if event_types.is_empty() || scanned.scanned_position <= position {
        return Ok(EventReadBatch::empty(scanned_cursor));
    }

    let mut query = QueryBuilder::<Sqlite>::new(
        r"
        SELECT event_type, occurred_at, payload_json
        FROM event_journal
        WHERE position > ",
    );
    query.push_bind(position);
    query.push(" AND position <= ");
    query.push_bind(scanned.scanned_position);
    query.push(" AND event_type IN (");
    let mut separated = query.separated(", ");
    for event_type in event_types {
        separated.push_bind(event_type);
    }
    separated.push_unseparated(") ORDER BY position");

    let rows = query
        .build_query_as::<JournalRow>()
        .fetch_all(&mut **tx)
        .await?;
    let events = rows
        .into_iter()
        .map(JournalRow::into_journaled_event)
        .collect::<SqliteResult<Vec<_>>>()?;

    Ok(EventReadBatch::new(events, scanned_cursor))
}

async fn load_cursor(
    tx: &mut Transaction<'_, Sqlite>,
    processor: ProcessorId,
) -> SqliteResult<EventCursor> {
    let row = sqlx::query_as::<_, CursorRow>(
        r"
        SELECT position
        FROM event_cursor
        WHERE consumer = ?
        ",
    )
    .bind(processor.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    let Some(row) = row else {
        return Ok(EventCursor::initial(processor));
    };
    Ok(EventCursor::at(
        processor,
        EventCursorPos::position(row.position.to_string()),
    ))
}

async fn advance_cursor(
    tx: &mut Transaction<'_, Sqlite>,
    cursor: &EventCursor,
) -> SqliteResult<()> {
    let position = decode_event_cursor_position(cursor.position())?;
    sqlx::query(
        r"
        INSERT INTO event_cursor (consumer, position)
        VALUES (?, ?)
        ON CONFLICT(consumer) DO UPDATE SET
            position = CASE
                WHEN excluded.position > event_cursor.position
                THEN excluded.position
                ELSE event_cursor.position
            END
        ",
    )
    .bind(cursor.processor().as_str())
    .bind(position)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

#[derive(sqlx::FromRow)]
struct ScannedPositionRow {
    scanned_position: i64,
}

#[derive(sqlx::FromRow)]
struct JournalRow {
    event_type: String,
    occurred_at: DateTime<Utc>,
    payload_json: String,
}

impl JournalRow {
    fn into_journaled_event(self) -> SqliteResult<JournaledEvent> {
        Ok(JournaledEvent::new(
            Event::decode(&self.event_type, &self.payload_json)?,
            self.occurred_at,
        ))
    }
}

#[derive(sqlx::FromRow)]
struct CursorRow {
    position: i64,
}

fn decode_event_cursor_position(position: &EventCursorPos) -> SqliteResult<i64> {
    match position {
        EventCursorPos::Initial => Ok(0),
        EventCursorPos::Position(position) => {
            let position = position.parse::<i64>().decode()?;
            if position < 0 {
                return Err(SqliteError::decode_message(format!(
                    "event cursor position must be non-negative: {position}"
                )));
            }
            Ok(position)
        }
    }
}

impl EventJournalAppend for SqliteRegistryTx<'_> {
    async fn append_event(
        &mut self,
        event: Event,
        occurred_at: DateTime<Utc>,
    ) -> RegistryDbResult<EventType> {
        append_event(&mut self.tx, event, occurred_at).await.db()
    }
}

impl EventJournal for SqliteRegistryTx<'_> {
    async fn read_after(
        &mut self,
        cursor: &EventCursor,
        interests: EventInterests,
    ) -> RegistryDbResult<EventReadBatch> {
        read_after(&mut self.tx, cursor, interests).await.db()
    }

    async fn load_cursor(&mut self, processor: ProcessorId) -> RegistryDbResult<EventCursor> {
        load_cursor(&mut self.tx, processor).await.db()
    }

    async fn advance_cursor(&mut self, cursor: &EventCursor) -> RegistryDbResult<()> {
        advance_cursor(&mut self.tx, cursor).await.db()
    }
}

#[cfg(test)]
mod tests;
