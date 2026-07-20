use sqlx::{Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    RegistryDbResult,
    db::FeedDb,
    feed::{UpsertFeedCommand, UpsertFeedOutcome},
};

use super::error::{IntoDbResult, SqliteError, SqliteResult};

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

async fn upsert_snapshot(
    tx: &mut Transaction<'_, Sqlite>,
    command: UpsertFeedCommand,
) -> SqliteResult<UpsertFeedOutcome> {
    let source = command.source;
    let feed_pk = resolve_pk(tx, &source.feed_url).await?;
    let meta_json = serde_json::to_string(&command.meta)?;
    let previous = load_snapshot(tx, feed_pk).await?;
    let outcome = match previous {
        None => {
            sqlx::query(
                r#"
                INSERT INTO feed_snapshot (feed_pk, meta_json, body_blob_pk)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(feed_pk)
            .bind(&meta_json)
            .bind(source.body_blob.pk())
            .execute(&mut **tx)
            .await?;
            UpsertFeedOutcome::Discovered
        }
        Some(previous) => {
            sqlx::query(
                r#"
                UPDATE feed_snapshot
                SET meta_json = ?, body_blob_pk = ?
                WHERE feed_pk = ?
                "#,
            )
            .bind(&meta_json)
            .bind(source.body_blob.pk())
            .bind(feed_pk)
            .execute(&mut **tx)
            .await?;
            if previous.meta_json != meta_json || previous.body_blob_pk != source.body_blob.pk() {
                UpsertFeedOutcome::Changed
            } else {
                UpsertFeedOutcome::Unchanged
            }
        }
    };
    Ok(outcome)
}

async fn load_snapshot(
    tx: &mut Transaction<'_, Sqlite>,
    feed_pk: i64,
) -> SqliteResult<Option<StoredSnapshot>> {
    let row = sqlx::query_as::<_, StoredSnapshot>(
        r#"
        SELECT meta_json, body_blob_pk
        FROM feed_snapshot
        WHERE feed_pk = ?
        "#,
    )
    .bind(feed_pk)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row)
}

#[derive(sqlx::FromRow)]
struct StoredSnapshot {
    meta_json: String,
    body_blob_pk: i64,
}

#[derive(sqlx::FromRow)]
struct PkRow {
    pk: i64,
}

impl FeedDb for super::SqliteRegistryTx<'_> {
    async fn upsert_feed(
        &mut self,
        command: UpsertFeedCommand,
    ) -> RegistryDbResult<UpsertFeedOutcome> {
        upsert_snapshot(&mut self.tx, command).await.db()
    }
}

#[cfg(test)]
mod tests;
