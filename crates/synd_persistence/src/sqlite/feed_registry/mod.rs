#![allow(clippy::needless_raw_string_hashes)]

use sqlx::{Row, Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    FeedRegistryDb, RegistryDbError, RegistryDbResult, RegistryDbTransaction, SubscriberId,
    Subscription,
    crawl::{
        state::{FeedSnapshot, RefreshFailure, RefreshStarted, RefreshState, RefreshSuccess},
        target_list::CrawlTarget,
    },
    event::{Event, EventEncoding},
    view::{Subscriptions, SubscriptionsQuery},
};

use self::codec::{
    decode_crawl_target, decode_refresh_state, decode_snapshot, decode_subscription, encode_policy,
    encode_polling_policy,
};
use super::{SqliteDatabase, SqliteEventJournal};

mod codec;

const SUBSCRIPTION_SELECT_COLUMNS: &str = r#"
subscriber_id,
feed_url,
requirement,
category,
refresh_policy_kind,
refresh_interval_seconds,
created_at,
updated_at
"#;

#[derive(Clone)]
pub struct SqliteFeedRegistryDb {
    db: SqliteDatabase,
}

pub struct SqliteRegistryDbTransaction<'a> {
    tx: Transaction<'a, Sqlite>,
}

impl SqliteFeedRegistryDb {
    pub fn new(db: SqliteDatabase) -> Self {
        Self { db }
    }

    pub fn event_journal(&self) -> SqliteEventJournal {
        SqliteEventJournal::new(self.db.clone())
    }
}

impl FeedRegistryDb for SqliteFeedRegistryDb {
    type Tx<'a> = SqliteRegistryDbTransaction<'a>;

    async fn begin(&self) -> Result<Self::Tx<'_>, RegistryDbError> {
        let tx = self.db.begin().await?;
        Ok(SqliteRegistryDbTransaction { tx })
    }
}

impl SqliteRegistryDbTransaction<'_> {
    async fn upsert_snapshot(&mut self, snapshot: FeedSnapshot) -> RegistryDbResult<()> {
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
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }
}

impl RegistryDbTransaction for SqliteRegistryDbTransaction<'_> {
    async fn append_event(&mut self, event: Event) -> RegistryDbResult<()> {
        let encoded = event.encode().map_err(RegistryDbError::internal)?;
        sqlx::query(
            r"
            INSERT INTO event_journal (event_type, payload_json)
            VALUES (?, ?)
            ",
        )
        .bind(encoded.event_type)
        .bind(encoded.payload_json)
        .execute(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn upsert_subscription(&mut self, subscription: Subscription) -> RegistryDbResult<()> {
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
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn delete_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<()> {
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
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn has_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<bool> {
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
        .map_err(RegistryDbError::internal)?;

        Ok(row.is_some())
    }

    async fn list_subscriptions(
        &mut self,
        query: SubscriptionsQuery,
    ) -> RegistryDbResult<Subscriptions> {
        let first = i64::try_from(query.first.saturating_add(1)).unwrap_or(i64::MAX);
        let rows = if let Some(after) = query.after {
            let sql = format!(
                r#"
                SELECT {SUBSCRIPTION_SELECT_COLUMNS}
                FROM feed_subscription
                WHERE subscriber_id = ? AND feed_url > ?
                ORDER BY feed_url
                LIMIT ?
                "#
            );
            sqlx::query(&sql)
                .bind(query.subscriber_id.as_str())
                .bind(after)
                .bind(first)
                .fetch_all(&mut *self.tx)
                .await
        } else {
            let sql = format!(
                r#"
                SELECT {SUBSCRIPTION_SELECT_COLUMNS}
                FROM feed_subscription
                WHERE subscriber_id = ?
                ORDER BY feed_url
                LIMIT ?
                "#
            );
            sqlx::query(&sql)
                .bind(query.subscriber_id.as_str())
                .bind(first)
                .fetch_all(&mut *self.tx)
                .await
        }
        .map_err(RegistryDbError::internal)?;

        let mut nodes = rows
            .iter()
            .map(decode_subscription)
            .collect::<RegistryDbResult<Vec<_>>>()?;
        let has_next_page = nodes.len() > query.first;
        if has_next_page {
            nodes.truncate(query.first);
        }
        let end_cursor = nodes.last().map(|sub| sub.feed_url.to_string());

        Ok(Subscriptions::from_subscriptions(
            nodes,
            has_next_page,
            end_cursor,
        ))
    }

    async fn list_active_subscriptions(&mut self) -> RegistryDbResult<Vec<Subscription>> {
        let sql = format!(
            r#"
            SELECT {SUBSCRIPTION_SELECT_COLUMNS}
            FROM feed_subscription
            ORDER BY feed_url
            "#
        );
        let rows = sqlx::query(&sql)
            .fetch_all(&mut *self.tx)
            .await
            .map_err(RegistryDbError::internal)?;

        rows.iter().map(decode_subscription).collect()
    }

    async fn list_active_subscriptions_for_feed(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Vec<Subscription>> {
        let sql = format!(
            r#"
            SELECT {SUBSCRIPTION_SELECT_COLUMNS}
            FROM feed_subscription
            WHERE feed_url = ?
            ORDER BY subscriber_id
            "#
        );
        let rows = sqlx::query(&sql)
            .bind(feed_url.as_str())
            .fetch_all(&mut *self.tx)
            .await
            .map_err(RegistryDbError::internal)?;

        rows.iter().map(decode_subscription).collect()
    }

    async fn list_subscriptions_for_subscriber(
        &mut self,
        subscriber_id: &SubscriberId,
    ) -> RegistryDbResult<Vec<Subscription>> {
        let sql = format!(
            r#"
            SELECT {SUBSCRIPTION_SELECT_COLUMNS}
            FROM feed_subscription
            WHERE subscriber_id = ?
            ORDER BY feed_url
            "#
        );
        let rows = sqlx::query(&sql)
            .bind(subscriber_id.as_str())
            .fetch_all(&mut *self.tx)
            .await
            .map_err(RegistryDbError::internal)?;

        rows.iter().map(decode_subscription).collect()
    }

    async fn list_active_feed_urls(&mut self) -> RegistryDbResult<Vec<FeedUrl>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT feed_url
            FROM feed_subscription
            ORDER BY feed_url
            "#,
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        rows.into_iter()
            .map(|row| {
                let url: String = row.try_get("feed_url").map_err(RegistryDbError::internal)?;
                FeedUrl::parse(&url).map_err(RegistryDbError::internal)
            })
            .collect()
    }

    async fn upsert_crawl_target(&mut self, target: CrawlTarget) -> RegistryDbResult<()> {
        let (is_active, polling_policy_kind, polling_interval_seconds) =
            match (target.is_active, target.polling_policy) {
                (true, Some(policy)) => {
                    let (kind, interval_seconds) = encode_polling_policy(policy);
                    (1_i64, Some(kind), interval_seconds)
                }
                (false, None) => (0_i64, None, None),
                (true, None) => {
                    return Err(RegistryDbError::internal(anyhow::anyhow!(
                        "active crawl target requires a polling policy"
                    )));
                }
                (false, Some(_)) => {
                    return Err(RegistryDbError::internal(anyhow::anyhow!(
                        "inactive crawl target must not have a polling policy"
                    )));
                }
            };

        sqlx::query(
            r#"
            INSERT INTO crawl_target (
                feed_url,
                is_active,
                polling_policy_kind,
                polling_interval_seconds,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(feed_url) DO UPDATE SET
                is_active = excluded.is_active,
                polling_policy_kind = excluded.polling_policy_kind,
                polling_interval_seconds = excluded.polling_interval_seconds,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(target.feed_url.as_str())
        .bind(is_active)
        .bind(polling_policy_kind)
        .bind(polling_interval_seconds)
        .bind(target.updated_at)
        .execute(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn load_crawl_target(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<CrawlTarget>> {
        let row = sqlx::query(
            r#"
            SELECT
                feed_url,
                is_active,
                polling_policy_kind,
                polling_interval_seconds,
                updated_at
            FROM crawl_target
            WHERE feed_url = ?
            "#,
        )
        .bind(feed_url.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        row.as_ref().map(decode_crawl_target).transpose()
    }

    async fn load_snapshots(
        &mut self,
        feed_urls: &[FeedUrl],
    ) -> RegistryDbResult<Vec<FeedSnapshot>> {
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
            .map_err(RegistryDbError::internal)?
            {
                snapshots.push(decode_snapshot(&row)?);
            }
        }
        Ok(snapshots)
    }

    async fn load_refresh_states(
        &mut self,
        feed_urls: &[FeedUrl],
    ) -> RegistryDbResult<Vec<RefreshState>> {
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
            .map_err(RegistryDbError::internal)?
            {
                states.push(decode_refresh_state(&row)?);
            }
        }
        Ok(states)
    }

    async fn delete_feed_state(&mut self, feed_url: &FeedUrl) -> RegistryDbResult<()> {
        sqlx::query(
            r#"
            DELETE FROM feed_refresh_state
            WHERE feed_url = ?
            "#,
        )
        .bind(feed_url.as_str())
        .execute(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        sqlx::query(
            r#"
            DELETE FROM feed_snapshot
            WHERE feed_url = ?
            "#,
        )
        .bind(feed_url.as_str())
        .execute(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn record_refresh_started(&mut self, event: RefreshStarted) -> RegistryDbResult<()> {
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
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn record_refresh_succeeded(&mut self, result: RefreshSuccess) -> RegistryDbResult<()> {
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
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn record_refresh_failed(&mut self, result: RefreshFailure) -> RegistryDbResult<()> {
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
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn commit(self) -> RegistryDbResult<()> {
        self.tx.commit().await.map_err(RegistryDbError::internal)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{TimeZone, Utc};
    use synd_feed::types::FeedUrl;
    use synd_registry::{
        FeedRegistry, FeedRegistryConfig, FeedRegistryRuntime, SubscribeFeedCommand,
        SubscriptionKey,
        crawl::{
            policy::{PollingSchedule, RefreshInterval, RefreshPolicy, RefreshSchedule},
            state::RefreshErrorKind,
            target_list::{CrawlTargetListInput, CrawlTargetListProj},
        },
        event::{
            ApiEvent, ApiEventPublisher, EventConsumer, EventConsumerId, EventConsumerSession,
            EventJournal, EventKind, EventReadFilter, EventRuntime, FeedSubscribed,
            FeedUnsubscribed, RequestEventKind, SubscriptionChanged, SubscriptionLifecycle,
        },
    };
    use tokio_util::sync::CancellationToken;

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

    async fn migrated_db() -> Result<SqliteFeedRegistryDb, RegistryDbError> {
        let db = SqliteDatabase::in_memory().await?;
        db.migrate().await?;
        Ok(SqliteFeedRegistryDb::new(db))
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

    fn subscription(path: &str) -> Subscription {
        subscription_with(
            subscriber_id(),
            path,
            RefreshPolicy::interval(interval(3600)),
        )
    }

    fn subscription_with(
        subscriber_id: SubscriberId,
        path: &str,
        refresh_policy: RefreshPolicy,
    ) -> Subscription {
        let now = Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap();
        Subscription {
            subscriber_id,
            feed_url: feed_url(path),
            requirement: None,
            category: None,
            refresh_policy,
            created_at: now,
            updated_at: now,
        }
    }

    fn subscription_key(subscription: &Subscription) -> SubscriptionKey {
        SubscriptionKey::new(
            subscription.subscriber_id.clone(),
            subscription.feed_url.clone(),
        )
    }

    async fn project_crawl_targets(
        db: &SqliteFeedRegistryDb,
        events: Vec<SubscriptionLifecycle>,
    ) -> anyhow::Result<()> {
        let journal = db.event_journal();
        let mut session = EventConsumerSession::new(&journal);
        let mut projector = CrawlTargetListProj::new(db.clone());
        projector
            .consume(CrawlTargetListInput::new(events), &mut session)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn subscribe_records_request_event() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let journal = db.event_journal();
        let registry = FeedRegistry::with_event_runtime(
            db.clone(),
            FeedRegistryConfig::default(),
            ApiEventPublisher::default(),
            EventRuntime::new(journal.clone()),
        );
        let subscriber_id = subscriber_id();
        let feed_url = feed_url("event");

        registry
            .subscribe(SubscribeFeedCommand {
                subscriber_id: subscriber_id.clone(),
                feed_url: feed_url.clone(),
                requirement: None,
                category: None,
                refresh_policy: RefreshPolicy::interval(interval(3600)),
            })
            .await?;

        let cursor = journal
            .load_cursor(EventConsumerId::SubRequestWorker)
            .await?;
        let batch = journal
            .read_after(
                &cursor,
                EventReadFilter::new(&[EventKind::Request(
                    RequestEventKind::SubscribeFeedRequested,
                )]),
            )
            .await?;

        assert_eq!(batch.events().len(), 1);
        assert_eq!(
            batch.events()[0].event().kind(),
            EventKind::Request(RequestEventKind::SubscribeFeedRequested)
        );
        Ok(())
    }

    #[tokio::test]
    async fn runtime_subscribe_projects_subscription_and_api_event() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let ct = CancellationToken::new();
        let runtime = FeedRegistryRuntime::start(
            db.clone(),
            db.event_journal(),
            FeedRegistryConfig {
                event_worker_poll_interval: Duration::from_millis(10),
                ..FeedRegistryConfig::default()
            },
            ct.clone(),
        );
        let registry = runtime.registry();
        let subscriber_id = subscriber_id();
        let feed_url = feed_url("runtime-subscribe");
        let mut api_events = registry.subscribe_api_events(subscriber_id.clone());

        let output = registry
            .subscribe(SubscribeFeedCommand {
                subscriber_id: subscriber_id.clone(),
                feed_url: feed_url.clone(),
                requirement: None,
                category: None,
                refresh_policy: RefreshPolicy::interval(interval(3600)),
            })
            .await?;

        let api_event = tokio::time::timeout(Duration::from_secs(2), api_events.recv()).await?;
        let api_event = match api_event {
            Ok(event) => event,
            Err(err) => anyhow::bail!("api event receive failed: {err:?}"),
        };
        let ApiEvent::FeedSubscribed(event) = api_event else {
            anyhow::bail!("unexpected api event: {api_event:?}");
        };
        assert_eq!(event.request_id, output.request_id);
        assert_eq!(event.subscription.subscriber_id, subscriber_id);
        assert_eq!(event.subscription.feed_url, feed_url);

        let page = registry
            .list_subscriptions(SubscriptionsQuery {
                subscriber_id,
                after: None,
                first: 10,
            })
            .await?;

        assert_eq!(page.subscriptions.len(), 1);
        assert_eq!(page.subscriptions[0].feed_url, feed_url);

        ct.cancel();
        drop(runtime);
        Ok(())
    }

    #[tokio::test]
    async fn uncommitted_subscription_is_rolled_back() -> Result<(), RegistryDbError> {
        let db = migrated_db().await?;
        {
            let mut tx = db.begin().await?;
            tx.upsert_subscription(subscription("rollback")).await?;
        }

        let mut tx = db.begin().await?;
        let page = tx
            .list_subscriptions(SubscriptionsQuery {
                subscriber_id: subscriber_id(),
                after: None,
                first: 10,
            })
            .await?;

        assert!(page.subscriptions.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn feed_scoped_subscription_read_uses_subscription_model() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("shared-read");

        let mut tx = db.begin().await?;
        tx.upsert_subscription(subscription.clone()).await?;
        tx.commit().await?;

        let mut tx = db.begin().await?;
        let page = tx
            .list_subscriptions(SubscriptionsQuery {
                subscriber_id: subscription.subscriber_id.clone(),
                after: None,
                first: 10,
            })
            .await?;
        let feed_subscriptions = tx
            .list_active_subscriptions_for_feed(&subscription.feed_url)
            .await?;

        assert_eq!(page.subscriptions, vec![subscription.clone()]);
        assert_eq!(feed_subscriptions, vec![subscription]);
        Ok(())
    }

    #[tokio::test]
    async fn crawl_target_projection_activates_target_after_subscription() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("crawl-target-subscribed");

        let mut tx = db.begin().await?;
        tx.upsert_subscription(subscription.clone()).await?;
        tx.commit().await?;

        project_crawl_targets(
            &db,
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribed::new(
                subscription_key(&subscription),
            ))],
        )
        .await?;

        let mut tx = db.begin().await?;
        let target = tx
            .load_crawl_target(&subscription.feed_url)
            .await?
            .expect("crawl target should be projected");

        assert!(target.is_active);
        assert_eq!(
            target
                .polling_policy
                .expect("active target has policy")
                .schedule,
            PollingSchedule::Interval(interval(3600))
        );
        Ok(())
    }

    #[tokio::test]
    async fn crawl_target_projection_aggregates_multiple_subscriptions() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let one_hour = subscription_with(
            SubscriberId::new("one-hour"),
            "crawl-target-aggregate",
            RefreshPolicy::interval(interval(3600)),
        );
        let ten_minutes = subscription_with(
            SubscriberId::new("ten-minutes"),
            "crawl-target-aggregate",
            RefreshPolicy::interval(interval(600)),
        );
        let manual = subscription_with(
            SubscriberId::new("manual"),
            "crawl-target-aggregate",
            RefreshPolicy {
                schedule: RefreshSchedule::Manual,
            },
        );

        let mut tx = db.begin().await?;
        tx.upsert_subscription(one_hour.clone()).await?;
        tx.upsert_subscription(ten_minutes).await?;
        tx.upsert_subscription(manual).await?;
        tx.commit().await?;

        project_crawl_targets(
            &db,
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribed::new(
                subscription_key(&one_hour),
            ))],
        )
        .await?;

        let mut tx = db.begin().await?;
        let target = tx
            .load_crawl_target(&one_hour.feed_url)
            .await?
            .expect("crawl target should be projected");

        assert!(target.is_active);
        assert_eq!(
            target
                .polling_policy
                .expect("active target has policy")
                .schedule,
            PollingSchedule::Interval(interval(600))
        );
        Ok(())
    }

    #[tokio::test]
    async fn crawl_target_projection_recalculates_after_subscription_change() -> anyhow::Result<()>
    {
        let db = migrated_db().await?;
        let mut subscription = subscription("crawl-target-changed");

        let mut tx = db.begin().await?;
        tx.upsert_subscription(subscription.clone()).await?;
        tx.commit().await?;

        project_crawl_targets(
            &db,
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribed::new(
                subscription_key(&subscription),
            ))],
        )
        .await?;

        subscription.refresh_policy = RefreshPolicy::interval(interval(300));
        let mut tx = db.begin().await?;
        tx.upsert_subscription(subscription.clone()).await?;
        tx.commit().await?;

        project_crawl_targets(
            &db,
            vec![SubscriptionLifecycle::Changed(SubscriptionChanged::new(
                subscription_key(&subscription),
            ))],
        )
        .await?;

        let mut tx = db.begin().await?;
        let target = tx
            .load_crawl_target(&subscription.feed_url)
            .await?
            .expect("crawl target should be projected");

        assert!(target.is_active);
        assert_eq!(
            target
                .polling_policy
                .expect("active target has policy")
                .schedule,
            PollingSchedule::Interval(interval(300))
        );
        Ok(())
    }

    #[tokio::test]
    async fn crawl_target_projection_inactivates_target_after_last_unsubscribe()
    -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("crawl-target-unsubscribed");

        let mut tx = db.begin().await?;
        tx.upsert_subscription(subscription.clone()).await?;
        tx.commit().await?;

        project_crawl_targets(
            &db,
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribed::new(
                subscription_key(&subscription),
            ))],
        )
        .await?;

        let mut tx = db.begin().await?;
        tx.delete_subscription(&subscription.subscriber_id, &subscription.feed_url)
            .await?;
        tx.commit().await?;

        project_crawl_targets(
            &db,
            vec![SubscriptionLifecycle::Unsubscribed(FeedUnsubscribed::new(
                subscription_key(&subscription),
            ))],
        )
        .await?;

        let mut tx = db.begin().await?;
        let target = tx
            .load_crawl_target(&subscription.feed_url)
            .await?
            .expect("crawl target should be projected");

        assert!(!target.is_active);
        assert_eq!(target.polling_policy, None);
        Ok(())
    }

    #[tokio::test]
    async fn refresh_success_writes_snapshot_and_state_atomically() -> Result<(), RegistryDbError> {
        let db = migrated_db().await?;
        let feed_url = feed_url("success");
        let succeeded_at = Utc.with_ymd_and_hms(2026, 5, 24, 12, 30, 0).unwrap();
        let next_refresh_after = Some(succeeded_at + chrono::Duration::hours(1));

        let mut tx = db.begin().await?;
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

        let mut tx = db.begin().await?;
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
    async fn refresh_records_do_not_recreate_state_without_subscription()
    -> Result<(), RegistryDbError> {
        let db = migrated_db().await?;
        let feed_url = feed_url("orphan");
        let happened_at = Utc.with_ymd_and_hms(2026, 5, 24, 12, 30, 0).unwrap();

        let mut tx = db.begin().await?;
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

        let mut tx = db.begin().await?;
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
