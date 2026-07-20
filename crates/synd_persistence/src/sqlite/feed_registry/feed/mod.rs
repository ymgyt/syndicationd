use std::{collections::HashMap, hash::BuildHasher, iter::FromIterator};

use sqlx::{Sqlite, Transaction};
use synd_feed::{
    entry::{Entry, EntryId},
    types::{Feed, FeedMeta, FeedUrl},
};
use synd_registry::{RegistryDbResult, db::FeedDb, entry::Entries, feed::FeedUpdate};

use super::{
    codec::{decode_stored_entry, decode_stored_feed_meta, encode_feed_meta_json},
    entry,
    error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
};

/// Registers the URL in the feed ledger and returns its pk.
pub(super) async fn upsert_pk(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
) -> SqliteResult<i64> {
    let row = sqlx::query_as::<_, PkRow>(
        r#"
            INSERT INTO feed (url)
            VALUES (?)
            ON CONFLICT(url) DO UPDATE SET
                url = excluded.url
            RETURNING pk
            "#,
    )
    .bind(feed_url.as_str())
    .fetch_one(&mut **tx)
    .await?;

    Ok(row.pk)
}

pub(super) async fn resolve_pk(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
) -> SqliteResult<i64> {
    let row = sqlx::query_as::<_, PkRow>(
        r#"
            SELECT pk
            FROM feed
            WHERE url = ?
            "#,
    )
    .bind(feed_url.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    row.map(|row| row.pk)
        .ok_or_else(|| SqliteError::not_found("feed", feed_url.as_str()))
}

async fn apply_update(tx: &mut Transaction<'_, Sqlite>, update: &FeedUpdate) -> SqliteResult<()> {
    let feed_pk = resolve_pk(tx, &update.source().feed_url).await?;
    upsert_snapshot(tx, feed_pk, update).await?;
    entry::apply_changes(tx, feed_pk, update.entry_changes()).await?;
    entry::sync_membership(tx, feed_pk, update.membership()).await?;
    Ok(())
}

async fn upsert_snapshot(
    tx: &mut Transaction<'_, Sqlite>,
    feed_pk: i64,
    update: &FeedUpdate,
) -> SqliteResult<()> {
    let meta_json = encode_feed_meta_json(update.meta())?;
    sqlx::query(
        r#"
            INSERT INTO feed_snapshot (feed_pk, meta_json, body_blob_pk)
            VALUES (?, ?, ?)
            ON CONFLICT(feed_pk) DO UPDATE SET
                meta_json = excluded.meta_json,
                body_blob_pk = excluded.body_blob_pk
            "#,
    )
    .bind(feed_pk)
    .bind(meta_json)
    .bind(update.source().body_blob.pk())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn load_feeds(
    tx: &mut Transaction<'_, Sqlite>,
    feed_urls: &[FeedUrl],
) -> SqliteResult<HashMap<FeedUrl, Feed>> {
    let Some(requested) = RequestedFeeds::encode(feed_urls)? else {
        return Ok(HashMap::new());
    };
    let stored = StoredFeeds::load(tx, &requested).await?;
    let complete = stored.load_entries(tx, &requested).await?;
    Ok(complete.into())
}

/// Stable JSON representation of one batch lookup request.
struct RequestedFeeds(String);

impl RequestedFeeds {
    fn encode(feed_urls: &[FeedUrl]) -> SqliteResult<Option<Self>> {
        if feed_urls.is_empty() {
            return Ok(None);
        }
        let feed_urls = feed_urls.iter().map(FeedUrl::as_str).collect::<Vec<_>>();
        Ok(Some(Self(serde_json::to_string(&feed_urls)?)))
    }

    fn as_json(&self) -> &str {
        &self.0
    }
}

/// Feed snapshots being reconstructed from their relational entry membership.
struct StoredFeeds(HashMap<FeedUrl, StoredFeed>);

impl StoredFeeds {
    async fn load(
        tx: &mut Transaction<'_, Sqlite>,
        requested: &RequestedFeeds,
    ) -> SqliteResult<Self> {
        let rows = sqlx::query_as::<_, FeedMetaRow>(
            r#"
                WITH requested(feed_url) AS (
                    SELECT DISTINCT CAST(value AS TEXT)
                    FROM json_each(?)
                )
                SELECT
                    f.url AS feed_url,
                    fs.meta_json
                FROM requested AS r
                INNER JOIN feed AS f
                    ON f.url = r.feed_url
                INNER JOIN feed_snapshot AS fs
                    ON fs.feed_pk = f.pk
                ORDER BY f.url
                "#,
        )
        .bind(requested.as_json())
        .fetch_all(&mut **tx)
        .await?;

        rows.into_iter()
            .map(StoredFeed::try_from)
            .collect::<SqliteResult<Self>>()
    }

    async fn load_entries(
        mut self,
        tx: &mut Transaction<'_, Sqlite>,
        requested: &RequestedFeeds,
    ) -> SqliteResult<Self> {
        let rows = sqlx::query_as::<_, FeedEntryRow>(
            r#"
                WITH requested(feed_url) AS (
                    SELECT DISTINCT CAST(value AS TEXT)
                    FROM json_each(?)
                )
                SELECT
                    f.url AS feed_url,
                    e.entry_id,
                    e.entry_json
                FROM requested AS r
                INNER JOIN feed AS f
                    ON f.url = r.feed_url
                INNER JOIN feed_entry AS fe
                    ON fe.feed_pk = f.pk
                INNER JOIN entry AS e
                    ON e.feed_pk = fe.feed_pk
                   AND e.entry_id = fe.entry_id
                ORDER BY f.url, e.order_time DESC, e.entry_id DESC
                "#,
        )
        .bind(requested.as_json())
        .fetch_all(&mut **tx)
        .await?;

        for entry in rows.into_iter().map(StoredFeedEntry::try_from) {
            self.push(entry?)?;
        }
        Ok(self)
    }

    fn push(&mut self, stored_entry: StoredFeedEntry) -> SqliteResult<()> {
        let feed = self.0.get_mut(&stored_entry.feed_url).ok_or_else(|| {
            SqliteError::not_found("feed snapshot", stored_entry.feed_url.as_str())
        })?;
        feed.entries.push(stored_entry.entry);
        Ok(())
    }
}

impl FromIterator<StoredFeed> for StoredFeeds {
    fn from_iter<T>(feeds: T) -> Self
    where
        T: IntoIterator<Item = StoredFeed>,
    {
        Self(
            feeds
                .into_iter()
                .map(|feed| (feed.meta.url().clone(), feed))
                .collect(),
        )
    }
}

impl<S> From<StoredFeeds> for HashMap<FeedUrl, Feed, S>
where
    S: BuildHasher + Default,
{
    fn from(stored: StoredFeeds) -> Self {
        stored
            .0
            .into_iter()
            .map(|(feed_url, feed)| (feed_url, Feed::from(feed)))
            .collect()
    }
}

#[derive(sqlx::FromRow)]
struct PkRow {
    pk: i64,
}

#[derive(sqlx::FromRow)]
struct FeedMetaRow {
    feed_url: String,
    meta_json: String,
}

#[derive(Debug)]
struct StoredFeed {
    meta: FeedMeta,
    entries: Vec<Entry>,
}

impl From<StoredFeed> for Feed {
    fn from(stored: StoredFeed) -> Self {
        Self::new(stored.meta, stored.entries)
    }
}

impl TryFrom<FeedMetaRow> for StoredFeed {
    type Error = SqliteError;

    fn try_from(row: FeedMetaRow) -> Result<Self, Self::Error> {
        Ok(Self {
            meta: decode_stored_feed_meta(&row.feed_url, &row.meta_json)?,
            entries: Vec::new(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct FeedEntryRow {
    feed_url: String,
    entry_id: String,
    entry_json: String,
}

struct StoredFeedEntry {
    feed_url: FeedUrl,
    entry: Entry,
}

impl TryFrom<FeedEntryRow> for StoredFeedEntry {
    type Error = SqliteError;

    fn try_from(row: FeedEntryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            feed_url: FeedUrl::parse(&row.feed_url).decode()?,
            entry: decode_stored_entry(&row.entry_id, &row.entry_json)?,
        })
    }
}

impl FeedDb for super::SqliteRegistryTx<'_> {
    async fn load_entries(&mut self, entry_ids: &[EntryId]) -> RegistryDbResult<Entries> {
        entry::load(&mut self.tx, entry_ids).await.db()
    }

    async fn apply_feed_update(&mut self, update: &FeedUpdate) -> RegistryDbResult<()> {
        apply_update(&mut self.tx, update).await.db()
    }

    async fn load_feeds(
        &mut self,
        feed_urls: &[FeedUrl],
    ) -> RegistryDbResult<HashMap<FeedUrl, Feed>> {
        load_feeds(&mut self.tx, feed_urls).await.db()
    }
}

#[cfg(test)]
mod tests;
