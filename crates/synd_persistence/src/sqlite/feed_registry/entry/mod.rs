use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::entry::{EntryId, EntryOrderKey, SyndEntry};
use synd_registry::entry::{Changes, Entries};

use super::{
    codec::{decode_stored_entry, encode_entry_json},
    error::{SqliteError, SqliteResult},
};

pub(super) async fn load(
    tx: &mut Transaction<'_, Sqlite>,
    entry_ids: &[EntryId],
) -> SqliteResult<Entries> {
    if entry_ids.is_empty() {
        return Ok(Entries::default());
    }

    let entry_ids = entry_ids.iter().map(EntryId::as_str).collect::<Vec<_>>();
    let entry_ids_json = serde_json::to_string(&entry_ids)?;
    let rows = sqlx::query_as::<_, EntryRow>(
        r#"
            WITH requested(entry_id) AS (
                SELECT DISTINCT CAST(value AS TEXT)
                FROM json_each(?)
            )
            SELECT
                e.entry_id,
                e.entry_json,
                e.order_time
            FROM requested AS r
            INNER JOIN entry AS e
                ON e.entry_id = r.entry_id
            ORDER BY e.entry_id
            "#,
    )
    .bind(entry_ids_json)
    .fetch_all(&mut **tx)
    .await?;

    rows.into_iter()
        .map(SyndEntry::try_from)
        .collect::<SqliteResult<Entries>>()
}

pub(super) async fn apply_changes(
    tx: &mut Transaction<'_, Sqlite>,
    feed_pk: i64,
    changes: &Changes,
) -> SqliteResult<()> {
    for change in changes {
        upsert(tx, feed_pk, change.entry()).await?;
    }
    Ok(())
}

pub(super) async fn sync_membership(
    tx: &mut Transaction<'_, Sqlite>,
    feed_pk: i64,
    membership: &[EntryId],
) -> SqliteResult<()> {
    let entry_ids = membership.iter().map(EntryId::as_str).collect::<Vec<_>>();
    let entry_ids_json = serde_json::to_string(&entry_ids)?;

    sqlx::query(
        r#"
            DELETE FROM feed_entry
            WHERE feed_pk = ?
              AND NOT EXISTS (
                  SELECT 1
                  FROM json_each(?) AS desired
                  WHERE CAST(desired.value AS TEXT) = feed_entry.entry_id
              )
            "#,
    )
    .bind(feed_pk)
    .bind(&entry_ids_json)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
            INSERT INTO feed_entry (feed_pk, entry_id)
            SELECT ?, CAST(value AS TEXT)
            FROM json_each(?)
            WHERE true
            ON CONFLICT(feed_pk, entry_id) DO NOTHING
            "#,
    )
    .bind(feed_pk)
    .bind(entry_ids_json)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn upsert(
    tx: &mut Transaction<'_, Sqlite>,
    feed_pk: i64,
    entry: &SyndEntry,
) -> SqliteResult<()> {
    let entry_json = encode_entry_json(entry.entry())?;
    let result = sqlx::query(
        r#"
            INSERT INTO entry (entry_id, feed_pk, entry_json, order_time)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(entry_id) DO UPDATE SET
                entry_json = excluded.entry_json
            WHERE entry.feed_pk = excluded.feed_pk
            "#,
    )
    .bind(entry.entry().id().as_str())
    .bind(feed_pk)
    .bind(entry_json)
    .bind(entry.order_key().as_datetime())
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() != 1 {
        return Err(SqliteError::EntryWriteCount {
            feed_pk,
            entry_id: entry.entry().id().clone(),
            rows_affected: result.rows_affected(),
        });
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    entry_id: String,
    entry_json: String,
    order_time: DateTime<Utc>,
}

impl TryFrom<EntryRow> for SyndEntry {
    type Error = SqliteError;

    fn try_from(row: EntryRow) -> Result<Self, Self::Error> {
        let entry = decode_stored_entry(&row.entry_id, &row.entry_json)?;
        Ok(Self::builder()
            .entry(entry)
            .order_key(EntryOrderKey::from_datetime(row.order_time))
            .build())
    }
}
