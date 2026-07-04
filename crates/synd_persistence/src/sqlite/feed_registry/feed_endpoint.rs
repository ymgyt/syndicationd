use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::FeedUrl;

use super::error::{SqliteError, SqliteResult};

pub(super) async fn upsert(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> SqliteResult<i64> {
    let row = sqlx::query_as::<_, PkRow>(
        r#"
        INSERT INTO feed_endpoint (
            url,
            created_at,
            updated_at
        )
        VALUES (?, ?, ?)
        ON CONFLICT(url) DO UPDATE SET
            url = excluded.url
        RETURNING pk
        "#,
    )
    .bind(feed_url.as_str())
    .bind(created_at)
    .bind(updated_at)
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
        FROM feed_endpoint
        WHERE url = ?
        "#,
    )
    .bind(feed_url.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    row.map(|row| row.pk)
        .ok_or_else(|| SqliteError::not_found("feed endpoint", feed_url.as_str()))
}

#[derive(sqlx::FromRow)]
struct PkRow {
    pk: i64,
}
