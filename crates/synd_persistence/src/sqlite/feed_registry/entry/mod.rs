use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::{entry::EntryId, types::FeedUrl};
use synd_registry::{
    RegistryDbResult,
    db::EntryStore,
    entry::{Entry, EntryChange, EntryChanges, EntryOrderKey, EntrySet},
};

use super::{
    codec::{decode_entry_attrs_json, encode_entry_attrs_json},
    error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
    feed,
};

async fn load_entries(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    entry_ids: &[EntryId],
) -> SqliteResult<EntrySet> {
    if entry_ids.is_empty() {
        return Ok(EntrySet::empty(feed_url.clone()));
    }

    let entry_id_values = entry_ids.iter().map(EntryId::as_str).collect::<Vec<_>>();
    let entry_ids_json = serde_json::to_string(&entry_id_values)?;
    let rows = sqlx::query_as::<_, EntryRow>(
        r#"
            WITH requested(entry_id) AS (
                SELECT CAST(value AS TEXT)
                FROM json_each(?)
            )
            SELECT
                e.entry_id,
                e.attrs_json,
                e.content,
                e.order_time
            FROM requested AS r
            INNER JOIN entry AS e
                ON e.entry_id = r.entry_id
            INNER JOIN feed AS f
                ON f.pk = e.feed_pk
            WHERE f.url = ?
            ORDER BY e.entry_id
            "#,
    )
    .bind(entry_ids_json)
    .bind(feed_url.as_str())
    .fetch_all(&mut **tx)
    .await?;

    let entries = rows
        .into_iter()
        .map(|row| row.into_entry(feed_url))
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(EntrySet::new(feed_url.clone(), entries))
}

async fn apply_entry_changes(
    tx: &mut Transaction<'_, Sqlite>,
    changes: EntryChanges,
) -> SqliteResult<()> {
    for change in changes.into_changes() {
        match change {
            EntryChange::Discovered(entry) => insert_entry(tx, &entry).await?,
            EntryChange::Changed(entry) => update_entry(tx, &entry).await?,
        }
    }
    Ok(())
}

async fn insert_entry(tx: &mut Transaction<'_, Sqlite>, entry: &Entry) -> SqliteResult<()> {
    let feed_pk = feed::resolve_pk(tx, &entry.feed_url).await?;
    let attrs_json = encode_entry_attrs_json(&entry.attrs)?;
    sqlx::query(
        r#"
            INSERT INTO entry (
                entry_id,
                feed_pk,
                attrs_json,
                content,
                order_time
            )
            VALUES (?, ?, ?, ?, ?)
            "#,
    )
    .bind(entry.id.as_str())
    .bind(feed_pk)
    .bind(attrs_json)
    .bind(entry.content.as_deref())
    .bind(entry.order_key.as_datetime())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

// order_time is immutable after discovery, so updates never touch it.
async fn update_entry(tx: &mut Transaction<'_, Sqlite>, entry: &Entry) -> SqliteResult<()> {
    let attrs_json = encode_entry_attrs_json(&entry.attrs)?;
    let result = sqlx::query(
        r#"
            UPDATE entry
            SET
                attrs_json = ?,
                content = ?
            WHERE entry_id = ?
            "#,
    )
    .bind(attrs_json)
    .bind(entry.content.as_deref())
    .bind(entry.id.as_str())
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() != 1 {
        return Err(SqliteError::decode_message(format!(
            "entry update affected {} rows for {}",
            result.rows_affected(),
            entry.id
        )));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    entry_id: String,
    attrs_json: String,
    content: Option<String>,
    order_time: DateTime<Utc>,
}

impl EntryRow {
    fn into_entry(self, feed_url: &FeedUrl) -> SqliteResult<Entry> {
        Ok(Entry::builder()
            .id(EntryId::parse(self.entry_id).decode()?)
            .feed_url(feed_url.clone())
            .attrs(decode_entry_attrs_json(&self.attrs_json)?)
            .maybe_content(self.content)
            .order_key(EntryOrderKey::from_datetime(self.order_time))
            .build())
    }
}

impl EntryStore for super::SqliteRegistryTx<'_> {
    async fn load_entries(
        &mut self,
        feed_url: &FeedUrl,
        entry_ids: &[EntryId],
    ) -> RegistryDbResult<EntrySet> {
        load_entries(&mut self.tx, feed_url, entry_ids).await.db()
    }

    async fn apply_entry_changes(&mut self, changes: EntryChanges) -> RegistryDbResult<()> {
        apply_entry_changes(&mut self.tx, changes).await.db()
    }
}

#[cfg(test)]
mod tests;
