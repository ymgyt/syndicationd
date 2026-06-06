use chrono::{DateTime, Utc};
use sqlx::{Row, Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{RegistryDbError, RegistryDbResult};

pub(super) struct FeedEndpointTable<'tx, 'db> {
    tx: &'tx mut Transaction<'db, Sqlite>,
}

impl<'tx, 'db> FeedEndpointTable<'tx, 'db> {
    pub(super) fn new(tx: &'tx mut Transaction<'db, Sqlite>) -> Self {
        Self { tx }
    }

    pub(super) async fn upsert(
        &mut self,
        feed_url: &FeedUrl,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> RegistryDbResult<i64> {
        let row = sqlx::query(
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
        .fetch_one(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        row.try_get("pk").map_err(RegistryDbError::internal)
    }

    pub(super) async fn resolve_pk(&mut self, feed_url: &FeedUrl) -> RegistryDbResult<i64> {
        let row = sqlx::query(
            r#"
            SELECT pk
            FROM feed_endpoint
            WHERE url = ?
            "#,
        )
        .bind(feed_url.as_str())
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let Some(row) = row else {
            return Err(RegistryDbError::internal(anyhow::anyhow!(
                "feed endpoint not found: {}",
                feed_url.as_str()
            )));
        };

        row.try_get("pk").map_err(RegistryDbError::internal)
    }

    pub(super) async fn resolve_url(&mut self, pk: i64) -> RegistryDbResult<FeedUrl> {
        let row = sqlx::query(
            r#"
            SELECT url
            FROM feed_endpoint
            WHERE pk = ?
            "#,
        )
        .bind(pk)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let Some(row) = row else {
            return Err(RegistryDbError::internal(anyhow::anyhow!(
                "feed endpoint not found: {pk}"
            )));
        };
        let url = row
            .try_get::<String, _>("url")
            .map_err(RegistryDbError::internal)?;
        FeedUrl::parse(&url).map_err(RegistryDbError::internal)
    }
}
