#![allow(clippy::needless_raw_string_hashes)]

use std::path::Path;

use sqlx::{Row, Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    FeedRegistryStore, RegistryTransaction, StoreError, StoreResult,
    model::{
        FeedSnapshot, FeedSubscription, FeedSubscriptionPage, ListSubscriptionsQuery,
        RefreshFailure, RefreshStarted, RefreshState, RefreshSuccess, SubscriberId,
    },
};

use self::codec::{decode_refresh_state, decode_snapshot, decode_subscription, encode_policy};
use super::SqliteDatabase;

mod codec;

#[derive(Clone)]
pub struct SqliteFeedRegistryStore {
    db: SqliteDatabase,
}

pub struct SqliteRegistryTransaction<'a> {
    tx: Transaction<'a, Sqlite>,
}

impl SqliteFeedRegistryStore {
    pub async fn open(db_path: impl AsRef<Path>) -> Result<Self, StoreError> {
        SqliteDatabase::open(db_path).await.map(Self::from_database)
    }

    pub async fn create_or_open(db_path: impl AsRef<Path>) -> Result<Self, StoreError> {
        SqliteDatabase::create_or_open(db_path)
            .await
            .map(Self::from_database)
    }

    pub async fn migrate(&self) -> Result<(), StoreError> {
        self.db.migrate().await
    }

    pub fn from_database(db: SqliteDatabase) -> Self {
        Self { db }
    }

    #[cfg(test)]
    pub async fn in_memory() -> Result<Self, StoreError> {
        SqliteDatabase::in_memory().await.map(Self::from_database)
    }
}

impl FeedRegistryStore for SqliteFeedRegistryStore {
    type Tx<'a> = SqliteRegistryTransaction<'a>;

    async fn begin(&self) -> Result<Self::Tx<'_>, StoreError> {
        let tx = self.db.begin().await?;
        Ok(SqliteRegistryTransaction { tx })
    }
}

impl SqliteRegistryTransaction<'_> {
    async fn upsert_snapshot(&mut self, snapshot: FeedSnapshot) -> StoreResult<()> {
        sqlx::query(
            r#"
            INSERT INTO feed_snapshot (
                feed_url,
                body,
                content_type,
                etag,
                last_modified,
                fetched_at
            )
            SELECT ?, ?, ?, ?, ?, ?
            WHERE EXISTS (
                SELECT 1 FROM feed_subscription WHERE feed_url = ?
            )
            ON CONFLICT(feed_url) DO UPDATE SET
                body = excluded.body,
                content_type = excluded.content_type,
                etag = excluded.etag,
                last_modified = excluded.last_modified,
                fetched_at = excluded.fetched_at
            "#,
        )
        .bind(snapshot.feed_url.as_str())
        .bind(snapshot.body)
        .bind(snapshot.content_type)
        .bind(snapshot.etag)
        .bind(snapshot.last_modified)
        .bind(snapshot.fetched_at)
        .bind(snapshot.feed_url.as_str())
        .execute(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        Ok(())
    }
}

impl RegistryTransaction for SqliteRegistryTransaction<'_> {
    async fn upsert_subscription(&mut self, subscription: FeedSubscription) -> StoreResult<()> {
        let requirement = subscription.requirement.map(|r| r.to_string());
        let category = subscription.category.map(|c| c.to_string());
        let (policy_kind, interval_seconds) = encode_policy(subscription.refresh_policy);

        sqlx::query(
            r#"
            INSERT INTO feed_subscription (
                subscriber_id,
                feed_url,
                requirement,
                category,
                refresh_policy_kind,
                refresh_interval_seconds,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(subscriber_id, feed_url) DO UPDATE SET
                requirement = excluded.requirement,
                category = excluded.category,
                refresh_policy_kind = excluded.refresh_policy_kind,
                refresh_interval_seconds = excluded.refresh_interval_seconds,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(subscription.subscriber_id.as_str())
        .bind(subscription.feed_url.as_str())
        .bind(requirement)
        .bind(category)
        .bind(policy_kind)
        .bind(interval_seconds)
        .bind(subscription.created_at)
        .bind(subscription.updated_at)
        .execute(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        Ok(())
    }

    async fn delete_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> StoreResult<()> {
        sqlx::query(
            r#"
            DELETE FROM feed_subscription
            WHERE subscriber_id = ? AND feed_url = ?
            "#,
        )
        .bind(subscriber_id.as_str())
        .bind(feed_url.as_str())
        .execute(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        Ok(())
    }

    async fn has_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> StoreResult<bool> {
        let row = sqlx::query(
            r#"
            SELECT 1 AS found
            FROM feed_subscription
            WHERE subscriber_id = ? AND feed_url = ?
            LIMIT 1
            "#,
        )
        .bind(subscriber_id.as_str())
        .bind(feed_url.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        Ok(row.is_some())
    }

    async fn list_subscriptions(
        &mut self,
        query: ListSubscriptionsQuery,
    ) -> StoreResult<FeedSubscriptionPage> {
        let first = i64::try_from(query.first.saturating_add(1)).unwrap_or(i64::MAX);
        let rows = if let Some(after) = query.after {
            sqlx::query(
                r#"
                SELECT
                    subscriber_id,
                    feed_url,
                    requirement,
                    category,
                    refresh_policy_kind,
                    refresh_interval_seconds,
                    created_at,
                    updated_at
                FROM feed_subscription
                WHERE subscriber_id = ? AND feed_url > ?
                ORDER BY feed_url
                LIMIT ?
                "#,
            )
            .bind(query.subscriber_id.as_str())
            .bind(after)
            .bind(first)
            .fetch_all(&mut *self.tx)
            .await
        } else {
            sqlx::query(
                r#"
                SELECT
                    subscriber_id,
                    feed_url,
                    requirement,
                    category,
                    refresh_policy_kind,
                    refresh_interval_seconds,
                    created_at,
                    updated_at
                FROM feed_subscription
                WHERE subscriber_id = ?
                ORDER BY feed_url
                LIMIT ?
                "#,
            )
            .bind(query.subscriber_id.as_str())
            .bind(first)
            .fetch_all(&mut *self.tx)
            .await
        }
        .map_err(StoreError::internal)?;

        let mut nodes = rows
            .iter()
            .map(decode_subscription)
            .collect::<StoreResult<Vec<_>>>()?;
        let has_next_page = nodes.len() > query.first;
        if has_next_page {
            nodes.truncate(query.first);
        }
        let end_cursor = nodes.last().map(|sub| sub.feed_url.to_string());

        Ok(FeedSubscriptionPage {
            nodes,
            has_next_page,
            end_cursor,
        })
    }

    async fn list_active_subscriptions(&mut self) -> StoreResult<Vec<FeedSubscription>> {
        let rows = sqlx::query(
            r#"
            SELECT
                subscriber_id,
                feed_url,
                requirement,
                category,
                refresh_policy_kind,
                refresh_interval_seconds,
                created_at,
                updated_at
            FROM feed_subscription
            ORDER BY feed_url
            "#,
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        rows.iter().map(decode_subscription).collect()
    }

    async fn list_active_subscriptions_for_feed(
        &mut self,
        feed_url: &FeedUrl,
    ) -> StoreResult<Vec<FeedSubscription>> {
        let rows = sqlx::query(
            r#"
            SELECT
                subscriber_id,
                feed_url,
                requirement,
                category,
                refresh_policy_kind,
                refresh_interval_seconds,
                created_at,
                updated_at
            FROM feed_subscription
            WHERE feed_url = ?
            ORDER BY subscriber_id
            "#,
        )
        .bind(feed_url.as_str())
        .fetch_all(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        rows.iter().map(decode_subscription).collect()
    }

    async fn list_subscriptions_for_subscriber(
        &mut self,
        subscriber_id: &SubscriberId,
    ) -> StoreResult<Vec<FeedSubscription>> {
        let rows = sqlx::query(
            r#"
            SELECT
                subscriber_id,
                feed_url,
                requirement,
                category,
                refresh_policy_kind,
                refresh_interval_seconds,
                created_at,
                updated_at
            FROM feed_subscription
            WHERE subscriber_id = ?
            ORDER BY feed_url
            "#,
        )
        .bind(subscriber_id.as_str())
        .fetch_all(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        rows.iter().map(decode_subscription).collect()
    }

    async fn list_active_feed_urls(&mut self) -> StoreResult<Vec<FeedUrl>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT feed_url
            FROM feed_subscription
            ORDER BY feed_url
            "#,
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        rows.into_iter()
            .map(|row| {
                let url: String = row.try_get("feed_url").map_err(StoreError::internal)?;
                FeedUrl::parse(&url).map_err(StoreError::internal)
            })
            .collect()
    }

    async fn load_snapshots(&mut self, feed_urls: &[FeedUrl]) -> StoreResult<Vec<FeedSnapshot>> {
        let mut snapshots = Vec::new();
        for feed_url in feed_urls {
            if let Some(row) = sqlx::query(
                r#"
                SELECT feed_url, body, content_type, etag, last_modified, fetched_at
                FROM feed_snapshot
                WHERE feed_url = ?
                "#,
            )
            .bind(feed_url.as_str())
            .fetch_optional(&mut *self.tx)
            .await
            .map_err(StoreError::internal)?
            {
                snapshots.push(decode_snapshot(&row)?);
            }
        }
        Ok(snapshots)
    }

    async fn load_refresh_states(
        &mut self,
        feed_urls: &[FeedUrl],
    ) -> StoreResult<Vec<RefreshState>> {
        let mut states = Vec::new();
        for feed_url in feed_urls {
            if let Some(row) = sqlx::query(
                r#"
                SELECT
                    feed_url,
                    last_attempt_at,
                    last_success_at,
                    last_failure_at,
                    last_error_kind,
                    last_error_message,
                    next_refresh_after
                FROM feed_refresh_state
                WHERE feed_url = ?
                "#,
            )
            .bind(feed_url.as_str())
            .fetch_optional(&mut *self.tx)
            .await
            .map_err(StoreError::internal)?
            {
                states.push(decode_refresh_state(&row)?);
            }
        }
        Ok(states)
    }

    async fn delete_feed_state(&mut self, feed_url: &FeedUrl) -> StoreResult<()> {
        sqlx::query(
            r#"
            DELETE FROM feed_refresh_state
            WHERE feed_url = ?
            "#,
        )
        .bind(feed_url.as_str())
        .execute(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        sqlx::query(
            r#"
            DELETE FROM feed_snapshot
            WHERE feed_url = ?
            "#,
        )
        .bind(feed_url.as_str())
        .execute(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        Ok(())
    }

    async fn record_refresh_started(&mut self, event: RefreshStarted) -> StoreResult<()> {
        sqlx::query(
            r#"
            INSERT INTO feed_refresh_state (feed_url, last_attempt_at)
            SELECT ?, ?
            WHERE EXISTS (
                SELECT 1 FROM feed_subscription WHERE feed_url = ?
            )
            ON CONFLICT(feed_url) DO UPDATE SET
                last_attempt_at = excluded.last_attempt_at
            "#,
        )
        .bind(event.feed_url.as_str())
        .bind(event.started_at)
        .bind(event.feed_url.as_str())
        .execute(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        Ok(())
    }

    async fn record_refresh_succeeded(&mut self, result: RefreshSuccess) -> StoreResult<()> {
        let feed_url = result.snapshot.feed_url.clone();
        self.upsert_snapshot(result.snapshot).await?;

        sqlx::query(
            r#"
            INSERT INTO feed_refresh_state (
                feed_url,
                last_attempt_at,
                last_success_at,
                last_error_kind,
                last_error_message,
                next_refresh_after
            )
            SELECT ?, ?, ?, NULL, NULL, ?
            WHERE EXISTS (
                SELECT 1 FROM feed_subscription WHERE feed_url = ?
            )
            ON CONFLICT(feed_url) DO UPDATE SET
                last_attempt_at = excluded.last_attempt_at,
                last_success_at = excluded.last_success_at,
                last_error_kind = NULL,
                last_error_message = NULL,
                next_refresh_after = excluded.next_refresh_after
            "#,
        )
        .bind(feed_url.as_str())
        .bind(result.succeeded_at)
        .bind(result.succeeded_at)
        .bind(result.next_refresh_after)
        .bind(feed_url.as_str())
        .execute(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        Ok(())
    }

    async fn record_refresh_failed(&mut self, result: RefreshFailure) -> StoreResult<()> {
        sqlx::query(
            r#"
            INSERT INTO feed_refresh_state (
                feed_url,
                last_attempt_at,
                last_failure_at,
                last_error_kind,
                last_error_message,
                next_refresh_after
            )
            SELECT ?, ?, ?, ?, ?, ?
            WHERE EXISTS (
                SELECT 1 FROM feed_subscription WHERE feed_url = ?
            )
            ON CONFLICT(feed_url) DO UPDATE SET
                last_attempt_at = excluded.last_attempt_at,
                last_failure_at = excluded.last_failure_at,
                last_error_kind = excluded.last_error_kind,
                last_error_message = excluded.last_error_message,
                next_refresh_after = excluded.next_refresh_after
            "#,
        )
        .bind(result.feed_url.as_str())
        .bind(result.failed_at)
        .bind(result.failed_at)
        .bind(result.error_kind.as_str())
        .bind(&result.error_message)
        .bind(result.next_refresh_after)
        .bind(result.feed_url.as_str())
        .execute(&mut *self.tx)
        .await
        .map_err(StoreError::internal)?;

        Ok(())
    }

    async fn commit(self) -> StoreResult<()> {
        self.tx.commit().await.map_err(StoreError::internal)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{TimeZone, Utc};
    use synd_feed::{
        feed::service::FeedService,
        types::{Feed, FeedUrl},
    };
    use synd_registry::{
        FeedProvider, FeedProviderError, FeedRegistry, FeedRegistryConfig, FetchedFeed,
        RefreshExecutorHandle,
        model::{
            InitialRefreshMode, RefreshErrorKind, RefreshInterval, RefreshPolicy,
            RefreshStatusKind, SubscribeFeedCommand, SubscribeFeedRefresh, UnsubscribeFeedCommand,
        },
    };

    use super::*;

    const ATOM_FEED: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Example Feed</title>
  <id>https://example.com/success.xml</id>
  <updated>2026-05-24T00:00:00Z</updated>
  <entry>
    <title>Hello</title>
    <id>https://example.com/posts/1</id>
    <updated>2026-05-24T00:00:00Z</updated>
    <link href="https://example.com/posts/1" />
    <summary>Hello from a persisted snapshot.</summary>
  </entry>
</feed>
"#;

    #[derive(Clone)]
    struct StaticFeedProvider {
        body: Vec<u8>,
    }

    impl StaticFeedProvider {
        fn new(body: impl Into<Vec<u8>>) -> Self {
            Self { body: body.into() }
        }
    }

    impl FeedProvider for StaticFeedProvider {
        async fn fetch(&self, feed_url: FeedUrl) -> Result<FetchedFeed, FeedProviderError> {
            let feed = Self::parse(feed_url.clone(), self.body.as_slice())?;
            Ok(FetchedFeed {
                feed_url: feed_url.clone(),
                feed,
                snapshot: FeedSnapshot {
                    feed_url,
                    body: self.body.clone(),
                    content_type: Some("application/atom+xml".to_owned()),
                    etag: None,
                    last_modified: None,
                    fetched_at: Utc::now(),
                },
            })
        }

        fn parse_snapshot(&self, snapshot: &FeedSnapshot) -> Result<Feed, FeedProviderError> {
            Self::parse(snapshot.feed_url.clone(), snapshot.body.as_slice())
        }
    }

    impl StaticFeedProvider {
        fn parse(feed_url: FeedUrl, body: &[u8]) -> Result<Feed, FeedProviderError> {
            FeedService::new("synd-persistence-test", 1024 * 1024)
                .parse(feed_url, body)
                .map_err(FeedProviderError::from)
        }
    }

    async fn migrated_store() -> Result<SqliteFeedRegistryStore, StoreError> {
        let store = SqliteFeedRegistryStore::in_memory().await?;
        store.migrate().await?;
        Ok(store)
    }

    fn feed_url(path: &str) -> FeedUrl {
        FeedUrl::parse(&format!("https://example.com/{path}.xml")).unwrap()
    }

    fn subscriber_id() -> SubscriberId {
        SubscriberId::new("local")
    }

    fn interval(seconds: u64) -> RefreshInterval {
        RefreshInterval::try_from(Duration::from_secs(seconds)).unwrap()
    }

    fn subscription(path: &str) -> FeedSubscription {
        let now = Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap();
        FeedSubscription {
            subscriber_id: subscriber_id(),
            feed_url: feed_url(path),
            requirement: None,
            category: None,
            refresh_policy: RefreshPolicy::interval(interval(3600)),
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn uncommitted_subscription_is_rolled_back() -> Result<(), StoreError> {
        let store = migrated_store().await?;
        {
            let mut tx = store.begin().await?;
            tx.upsert_subscription(subscription("rollback")).await?;
        }

        let mut tx = store.begin().await?;
        let page = tx
            .list_subscriptions(ListSubscriptionsQuery {
                subscriber_id: subscriber_id(),
                after: None,
                first: 10,
            })
            .await?;

        assert!(page.nodes.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn refresh_success_writes_snapshot_and_state_atomically() -> Result<(), StoreError> {
        let store = migrated_store().await?;
        let feed_url = feed_url("success");
        let succeeded_at = Utc.with_ymd_and_hms(2026, 5, 24, 12, 30, 0).unwrap();
        let next_refresh_after = Some(succeeded_at + chrono::Duration::hours(1));

        let mut tx = store.begin().await?;
        tx.upsert_subscription(subscription("success")).await?;
        tx.record_refresh_succeeded(RefreshSuccess {
            snapshot: FeedSnapshot {
                feed_url: feed_url.clone(),
                body: br#"{"version":"https://jsonfeed.org/version/1.1"}"#.to_vec(),
                content_type: Some("application/feed+json".to_owned()),
                etag: Some(r#""feed-v1""#.to_owned()),
                last_modified: Some("Sun, 24 May 2026 12:00:00 GMT".to_owned()),
                fetched_at: succeeded_at,
            },
            succeeded_at,
            next_refresh_after,
        })
        .await?;
        tx.commit().await?;

        let mut tx = store.begin().await?;
        let snapshots = tx.load_snapshots(std::slice::from_ref(&feed_url)).await?;
        let states = tx
            .load_refresh_states(std::slice::from_ref(&feed_url))
            .await?;

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].feed_url, feed_url);
        assert_eq!(
            snapshots[0].content_type.as_deref(),
            Some("application/feed+json")
        );
        assert_eq!(snapshots[0].etag.as_deref(), Some(r#""feed-v1""#));
        assert_eq!(snapshots[0].fetched_at, succeeded_at);

        assert_eq!(states.len(), 1);
        assert_eq!(states[0].last_attempt_at, Some(succeeded_at));
        assert_eq!(states[0].last_success_at, Some(succeeded_at));
        assert_eq!(states[0].last_failure_at, None);
        assert_eq!(states[0].last_error_kind, None);
        assert_eq!(states[0].last_error_message, None);
        assert_eq!(states[0].next_refresh_after, next_refresh_after);

        Ok(())
    }

    #[tokio::test]
    async fn refresh_records_do_not_recreate_state_without_subscription() -> Result<(), StoreError>
    {
        let store = migrated_store().await?;
        let feed_url = feed_url("orphan");
        let happened_at = Utc.with_ymd_and_hms(2026, 5, 24, 12, 30, 0).unwrap();

        let mut tx = store.begin().await?;
        tx.record_refresh_started(RefreshStarted {
            feed_url: feed_url.clone(),
            started_at: happened_at,
        })
        .await?;
        tx.record_refresh_succeeded(RefreshSuccess {
            snapshot: FeedSnapshot {
                feed_url: feed_url.clone(),
                body: ATOM_FEED.as_bytes().to_vec(),
                content_type: Some("application/atom+xml".to_owned()),
                etag: None,
                last_modified: None,
                fetched_at: happened_at,
            },
            succeeded_at: happened_at,
            next_refresh_after: None,
        })
        .await?;
        tx.record_refresh_failed(RefreshFailure {
            feed_url: feed_url.clone(),
            failed_at: happened_at,
            error_kind: RefreshErrorKind::Fetch,
            error_message: "network error".to_owned(),
            next_refresh_after: None,
        })
        .await?;
        tx.commit().await?;

        let mut tx = store.begin().await?;
        assert!(
            tx.load_snapshots(std::slice::from_ref(&feed_url))
                .await?
                .is_empty()
        );
        assert!(
            tx.load_refresh_states(std::slice::from_ref(&feed_url))
                .await?
                .is_empty()
        );

        Ok(())
    }

    #[tokio::test]
    async fn require_success_subscription_persists_entries_before_returning() -> anyhow::Result<()>
    {
        let store = migrated_store().await?;
        let registry = FeedRegistry::new(
            store.clone(),
            StaticFeedProvider::new(ATOM_FEED),
            RefreshExecutorHandle::new(),
            FeedRegistryConfig::default(),
        );
        let subscriber_id = subscriber_id();
        let feed_url = feed_url("success");

        let output = registry
            .subscribe(SubscribeFeedCommand {
                subscriber_id: subscriber_id.clone(),
                feed_url: feed_url.clone(),
                requirement: None,
                category: None,
                refresh_policy: RefreshPolicy::interval(interval(3600)),
                initial_refresh: InitialRefreshMode::RequireSuccess,
            })
            .await?;

        assert!(matches!(
            output.refresh,
            SubscribeFeedRefresh::Completed(status)
                if status.feed_url == feed_url && status.kind == RefreshStatusKind::Idle
        ));

        let entries = registry
            .list_entries(synd_registry::ListEntriesQuery {
                subscriber_id: subscriber_id.clone(),
                after: None,
                first: 10,
            })
            .await?;

        assert_eq!(entries.nodes.len(), 1);
        assert_eq!(entries.nodes[0].entry.title(), Some("Hello"));

        registry
            .unsubscribe(UnsubscribeFeedCommand {
                subscriber_id,
                feed_url: feed_url.clone(),
            })
            .await?;

        let mut tx = store.begin().await?;
        assert!(
            tx.load_snapshots(std::slice::from_ref(&feed_url))
                .await?
                .is_empty()
        );
        assert!(
            tx.load_refresh_states(std::slice::from_ref(&feed_url))
                .await?
                .is_empty()
        );

        Ok(())
    }
}
