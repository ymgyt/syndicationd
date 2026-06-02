#![allow(clippy::needless_raw_string_hashes)]

use sqlx::{QueryBuilder, Row, Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    CommitTx, FeedRegistryDb, RegistryDbError, RegistryDbResult, RegistryTx, SubscriberId,
    Subscription,
    crawl::target_list::CrawlTarget,
    event::{
        Event, EventCursor, EventCursorPos, EventEncoding, EventInterests, EventReadBatch,
        JournalTx, JournaledEvent, ProcessorId,
    },
    query::{Subscriptions, SubscriptionsQuery},
};

use self::codec::{decode_crawl_target, decode_subscription, encode_crawl_policy_json};
use super::SqliteDatabase;

mod codec;

const SUBSCRIPTION_SELECT_COLUMNS: &str = r#"
s.subscriber_id AS subscriber_id,
e.url AS feed_url,
s.requirement AS requirement,
s.category AS category,
s.crawl_policy_json AS crawl_policy_json,
s.created_at AS created_at,
s.updated_at AS updated_at
"#;

/// SQLite-backed registry database handle.
#[derive(Clone)]
pub struct SqliteFeedRegistryDb {
    db: SqliteDatabase,
}

/// `SQLite` transaction used to atomically update registry state and event progress.
pub struct SqliteRegistryTx<'a> {
    tx: Transaction<'a, Sqlite>,
}

impl SqliteFeedRegistryDb {
    pub fn new(db: SqliteDatabase) -> Self {
        Self { db }
    }
}

impl FeedRegistryDb for SqliteFeedRegistryDb {
    type Tx<'a> = SqliteRegistryTx<'a>;

    async fn begin(&self) -> Result<Self::Tx<'_>, RegistryDbError> {
        let tx = self.db.begin().await?;
        Ok(SqliteRegistryTx { tx })
    }
}

impl JournalTx for SqliteRegistryTx<'_> {
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

    async fn read_after(
        &mut self,
        cursor: &EventCursor,
        interests: EventInterests,
    ) -> RegistryDbResult<EventReadBatch> {
        let position = decode_event_cursor_position(cursor.position())?;
        let processor = cursor.processor();
        let event_types = interests
            .kinds()
            .iter()
            .copied()
            .map(synd_registry::event::EventKind::event_type)
            .collect::<Vec<_>>();

        let scanned_position = sqlx::query(
            r"
            SELECT COALESCE(MAX(position), ?) AS scanned_position
            FROM event_journal
            WHERE position > ?
            ",
        )
        .bind(position)
        .bind(position)
        .fetch_one(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?
        .try_get::<i64, _>("scanned_position")
        .map_err(RegistryDbError::internal)?;

        let scanned_cursor = EventCursor::at(
            processor,
            EventCursorPos::position(scanned_position.to_string()),
        );

        if event_types.is_empty() || scanned_position <= position {
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

        let rows = query
            .build()
            .fetch_all(&mut *self.tx)
            .await
            .map_err(RegistryDbError::internal)?;
        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let position = row
                .try_get::<i64, _>("position")
                .map_err(RegistryDbError::internal)?;
            let event_type = row
                .try_get::<String, _>("event_type")
                .map_err(RegistryDbError::internal)?;
            let payload_json = row
                .try_get::<String, _>("payload_json")
                .map_err(RegistryDbError::internal)?;
            events.push(JournaledEvent::new(
                EventCursor::at(processor, EventCursorPos::position(position.to_string())),
                Event::decode(&event_type, &payload_json).map_err(RegistryDbError::internal)?,
            ));
        }

        Ok(EventReadBatch::new(events, scanned_cursor))
    }

    async fn load_cursor(&mut self, processor: ProcessorId) -> RegistryDbResult<EventCursor> {
        let row = sqlx::query(
            r"
            SELECT position
            FROM event_cursor
            WHERE consumer = ?
            ",
        )
        .bind(processor.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let Some(row) = row else {
            return Ok(EventCursor::initial(processor));
        };
        let position = row
            .try_get::<i64, _>("position")
            .map_err(RegistryDbError::internal)?;
        Ok(EventCursor::at(
            processor,
            EventCursorPos::position(position.to_string()),
        ))
    }

    async fn advance_cursor(&mut self, cursor: &EventCursor) -> RegistryDbResult<()> {
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
        .execute(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }
}

impl SqliteRegistryTx<'_> {
    async fn upsert_feed_endpoint_row(
        &mut self,
        feed_url: &FeedUrl,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
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
        .fetch_one(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        row.try_get("pk").map_err(RegistryDbError::internal)
    }

    async fn resolve_feed_endpoint_pk(&mut self, feed_url: &FeedUrl) -> RegistryDbResult<i64> {
        let row = sqlx::query(
            r#"
            SELECT pk
            FROM feed_endpoint
            WHERE url = ?
            "#,
        )
        .bind(feed_url.as_str())
        .fetch_optional(&mut *self.tx)
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
}

impl RegistryTx for SqliteRegistryTx<'_> {
    async fn upsert_feed_endpoint(
        &mut self,
        feed_url: &FeedUrl,
        now: chrono::DateTime<chrono::Utc>,
    ) -> RegistryDbResult<()> {
        self.upsert_feed_endpoint_row(feed_url, now, now).await?;
        Ok(())
    }

    async fn upsert_feed_subscription(
        &mut self,
        subscription: Subscription,
    ) -> RegistryDbResult<()> {
        let requirement = subscription.requirement.map(|r| r.to_string());
        let category = subscription.category.map(|c| c.to_string());
        let policy_json = encode_crawl_policy_json(subscription.crawl_policy)?;
        let feed_endpoint_pk = self
            .resolve_feed_endpoint_pk(&subscription.feed_url)
            .await?;

        sqlx::query(
            r#"
            INSERT INTO feed_endpoint_subscription (
                subscriber_id,
                feed_endpoint_pk,
                requirement,
                category,
                crawl_policy_json,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(subscriber_id, feed_endpoint_pk) DO UPDATE SET
                requirement = excluded.requirement,
                category = excluded.category,
                crawl_policy_json = excluded.crawl_policy_json,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(subscription.subscriber_id.as_str())
        .bind(feed_endpoint_pk)
        .bind(requirement)
        .bind(category)
        .bind(policy_json)
        .bind(subscription.created_at)
        .bind(subscription.updated_at)
        .execute(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn delete_feed_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<()> {
        sqlx::query(
            r#"
            DELETE FROM feed_endpoint_subscription
            WHERE subscriber_id = ?
              AND feed_endpoint_pk = (
                  SELECT pk
                  FROM feed_endpoint
                  WHERE url = ?
              )
            "#,
        )
        .bind(subscriber_id.as_str())
        .bind(feed_url.as_str())
        .execute(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn has_feed_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<bool> {
        let row = sqlx::query(
            r#"
            SELECT 1 AS found
            FROM feed_endpoint_subscription AS s
            INNER JOIN feed_endpoint AS e
                ON e.pk = s.feed_endpoint_pk
            WHERE s.subscriber_id = ? AND e.url = ?
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
                FROM feed_endpoint_subscription AS s
                INNER JOIN feed_endpoint AS e
                    ON e.pk = s.feed_endpoint_pk
                WHERE s.subscriber_id = ? AND e.url > ?
                ORDER BY e.url
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
                FROM feed_endpoint_subscription AS s
                INNER JOIN feed_endpoint AS e
                    ON e.pk = s.feed_endpoint_pk
                WHERE s.subscriber_id = ?
                ORDER BY e.url
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

    async fn list_active_subscriptions_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Vec<Subscription>> {
        let sql = format!(
            r#"
            SELECT {SUBSCRIPTION_SELECT_COLUMNS}
            FROM feed_endpoint_subscription AS s
            INNER JOIN feed_endpoint AS e
                ON e.pk = s.feed_endpoint_pk
            WHERE e.url = ?
            ORDER BY s.subscriber_id
            "#
        );
        let rows = sqlx::query(&sql)
            .bind(feed_url.as_str())
            .fetch_all(&mut *self.tx)
            .await
            .map_err(RegistryDbError::internal)?;

        rows.iter().map(decode_subscription).collect()
    }

    async fn upsert_crawl_target(&mut self, target: CrawlTarget) -> RegistryDbResult<()> {
        let feed_endpoint_pk = self.resolve_feed_endpoint_pk(&target.feed_url).await?;
        let (state, effective_policy_json) = match (target.is_active, target.crawl_policy) {
            (true, Some(policy)) => {
                let policy_json = encode_crawl_policy_json(policy)?;
                ("active", Some(policy_json))
            }
            (false, None) => ("inactive", None),
            (true, None) => {
                return Err(RegistryDbError::internal(anyhow::anyhow!(
                    "active crawl target requires an effective policy"
                )));
            }
            (false, Some(_)) => {
                return Err(RegistryDbError::internal(anyhow::anyhow!(
                    "inactive crawl target must not have an effective policy"
                )));
            }
        };
        let subscription_count =
            i64::try_from(target.subscription_count).map_err(RegistryDbError::internal)?;

        sqlx::query(
            r#"
            INSERT INTO crawl_target (
                feed_endpoint_pk,
                state,
                subscription_count,
                effective_policy_json,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(feed_endpoint_pk) DO UPDATE SET
                state = excluded.state,
                subscription_count = excluded.subscription_count,
                effective_policy_json = excluded.effective_policy_json,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(feed_endpoint_pk)
        .bind(state)
        .bind(subscription_count)
        .bind(effective_policy_json)
        .bind(target.created_at)
        .bind(target.updated_at)
        .execute(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn load_crawl_target_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<CrawlTarget>> {
        let row = sqlx::query(
            r#"
            SELECT
                e.url AS feed_url,
                ct.state AS state,
                ct.subscription_count AS subscription_count,
                ct.effective_policy_json AS effective_policy_json,
                ct.created_at AS created_at,
                ct.updated_at AS updated_at
            FROM crawl_target AS ct
            INNER JOIN feed_endpoint AS e
                ON e.pk = ct.feed_endpoint_pk
            WHERE e.url = ?
            "#,
        )
        .bind(feed_url.as_str())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        row.as_ref().map(decode_crawl_target).transpose()
    }
}

impl CommitTx for SqliteRegistryTx<'_> {
    async fn commit(self) -> RegistryDbResult<()> {
        self.tx.commit().await.map_err(RegistryDbError::internal)
    }
}

fn decode_event_cursor_position(position: &EventCursorPos) -> RegistryDbResult<i64> {
    match position {
        EventCursorPos::Initial => Ok(0),
        EventCursorPos::Position(position) => {
            let position = position.parse::<i64>().map_err(RegistryDbError::internal)?;
            if position < 0 {
                return Err(RegistryDbError::internal(anyhow::anyhow!(
                    "event cursor position must be non-negative: {position}"
                )));
            }
            Ok(position)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{TimeZone, Utc};
    use synd_feed::types::FeedUrl;
    use synd_registry::{
        FeedRegistry, FeedRegistryConfig, RegistryService, SubscribeFeedCommand, SubscriptionKey,
        crawl::{
            policy::{CrawlPolicy, PollingInterval, PollingPolicy},
            target_list::{CrawlTargetListInput, CrawlTargetListProj},
        },
        event::{
            ApiEvent, ConsumeContext, Consumer, EventInterests, EventSubmitter, EventWakePublisher,
            FeedSubscribed, FeedUnsubscribed, ProcessorId, RequestEventKind, SubEvent,
            SubEventKind, SubscriptionChanged, SubscriptionLifecycle,
        },
    };
    use tokio_util::sync::CancellationToken;

    use super::*;

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

    fn interval(seconds: u64) -> PollingInterval {
        PollingInterval::try_from(Duration::from_secs(seconds)).unwrap()
    }

    fn subscription(path: &str) -> Subscription {
        subscription_with(subscriber_id(), path, CrawlPolicy::interval(interval(3600)))
    }

    fn subscription_with(
        subscriber_id: SubscriberId,
        path: &str,
        crawl_policy: CrawlPolicy,
    ) -> Subscription {
        let now = Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap();
        Subscription {
            subscriber_id,
            feed_url: feed_url(path),
            requirement: None,
            category: None,
            crawl_policy,
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

    async fn store_subscription(
        tx: &mut SqliteRegistryTx<'_>,
        subscription: Subscription,
    ) -> RegistryDbResult<()> {
        tx.upsert_feed_endpoint(&subscription.feed_url, subscription.created_at)
            .await?;
        tx.upsert_feed_subscription(subscription).await
    }

    fn subscribed_event(path: &str) -> Event {
        Event::Sub(SubEvent::FeedSubscribed(FeedSubscribed::new(
            subscription_key(&subscription(path)),
        )))
    }

    fn changed_event(path: &str) -> Event {
        Event::Sub(SubEvent::SubscriptionChanged(SubscriptionChanged::new(
            subscription_key(&subscription(path)),
        )))
    }

    fn subscription_lifecycle_interests() -> EventInterests {
        EventInterests::new([
            SubEventKind::FeedSubscribed.into(),
            SubEventKind::SubscriptionChanged.into(),
            SubEventKind::FeedUnsubscribed.into(),
        ])
    }

    async fn project_crawl_targets(
        db: &SqliteFeedRegistryDb,
        events: Vec<SubscriptionLifecycle>,
    ) -> anyhow::Result<()> {
        let mut projector = CrawlTargetListProj::new();
        let mut tx = db.begin().await?;
        {
            let mut cx = ConsumeContext::new(&mut tx);
            for event in events {
                <CrawlTargetListProj as Consumer<SqliteFeedRegistryDb>>::consume(
                    &mut projector,
                    &mut cx,
                    CrawlTargetListInput::new(event),
                )
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    #[tokio::test]
    async fn load_cursor_returns_initial_cursor_for_new_processor() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let mut tx = db.begin().await?;

        let cursor = tx.load_cursor(ProcessorId::CrawlTargetProjection).await?;
        tx.commit().await?;

        assert_eq!(
            cursor,
            EventCursor::initial(ProcessorId::CrawlTargetProjection)
        );
        Ok(())
    }

    #[tokio::test]
    async fn append_and_read_subscription_events_for_processor() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        {
            let mut tx = db.begin().await?;
            tx.append_event(subscribed_event("subscribed")).await?;
            tx.append_event(changed_event("changed")).await?;
            tx.commit().await?;
        }

        let mut tx = db.begin().await?;
        let cursor = tx.load_cursor(ProcessorId::CrawlTargetProjection).await?;
        let batch = tx
            .read_after(&cursor, subscription_lifecycle_interests())
            .await?;
        tx.commit().await?;

        assert_eq!(batch.events().len(), 2);
        assert_eq!(batch.events()[0].event(), &subscribed_event("subscribed"));
        assert_eq!(batch.events()[1].event(), &changed_event("changed"));
        assert_eq!(
            batch.scanned_cursor(),
            &EventCursor::at(
                ProcessorId::CrawlTargetProjection,
                EventCursorPos::position("2")
            )
        );
        Ok(())
    }

    #[tokio::test]
    async fn subscribe_records_request_event() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let config = FeedRegistryConfig::default();
        let event_submitter = EventSubmitter::new(
            db.clone(),
            EventWakePublisher::new(config.event_wake_channel_capacity),
        );
        let registry = FeedRegistry::new(db.clone(), config, event_submitter);
        let subscriber_id = subscriber_id();
        let feed_url = feed_url("event");

        registry
            .subscribe(SubscribeFeedCommand {
                subscriber_id: subscriber_id.clone(),
                feed_url: feed_url.clone(),
                requirement: None,
                category: None,
                crawl_policy: CrawlPolicy::interval(interval(3600)),
            })
            .await?;

        let mut tx = db.begin().await?;
        let cursor = tx.load_cursor(ProcessorId::SubscriptionRequest).await?;
        let batch = tx
            .read_after(
                &cursor,
                EventInterests::new([RequestEventKind::SubscribeFeedRequested.into()]),
            )
            .await?;
        tx.commit().await?;

        assert_eq!(batch.events().len(), 1);
        assert_eq!(
            batch.events()[0].event().kind(),
            RequestEventKind::SubscribeFeedRequested.into()
        );
        Ok(())
    }

    #[tokio::test]
    async fn runtime_subscribe_projects_subscription_and_api_event() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let ct = CancellationToken::new();
        let config = FeedRegistryConfig {
            event_worker_poll_interval: Duration::from_millis(10),
            ..FeedRegistryConfig::default()
        };
        let registry_service = RegistryService::start(db.clone(), config, ct.clone());
        let (registry, event_workers) = registry_service.into_parts();
        let subscriber_id = subscriber_id();
        let feed_url = feed_url("runtime-subscribe");
        let mut api_events = registry.subscribe_api_events(subscriber_id.clone());

        let output = registry
            .subscribe(SubscribeFeedCommand {
                subscriber_id: subscriber_id.clone(),
                feed_url: feed_url.clone(),
                requirement: None,
                category: None,
                crawl_policy: CrawlPolicy::interval(interval(3600)),
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
        drop(event_workers);
        Ok(())
    }

    #[tokio::test]
    async fn uncommitted_subscription_is_rolled_back() -> Result<(), RegistryDbError> {
        let db = migrated_db().await?;
        {
            let mut tx = db.begin().await?;
            store_subscription(&mut tx, subscription("rollback")).await?;
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
    async fn feed_subscription_reads_are_backed_by_feed_endpoint() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("shared-read");

        let mut tx = db.begin().await?;
        store_subscription(&mut tx, subscription.clone()).await?;
        tx.commit().await?;

        let mut tx = db.begin().await?;
        let page = tx
            .list_subscriptions(SubscriptionsQuery {
                subscriber_id: subscription.subscriber_id.clone(),
                after: None,
                first: 10,
            })
            .await?;
        let endpoint_subscriptions = tx
            .list_active_subscriptions_for_endpoint(&subscription.feed_url)
            .await?;

        assert_eq!(page.subscriptions, vec![subscription.clone()]);
        assert_eq!(endpoint_subscriptions, vec![subscription]);
        Ok(())
    }

    #[tokio::test]
    async fn crawl_target_projection_activates_target_after_subscription() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("crawl-target-subscribed");

        let mut tx = db.begin().await?;
        store_subscription(&mut tx, subscription.clone()).await?;
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
            .load_crawl_target_for_endpoint(&subscription.feed_url)
            .await?
            .expect("crawl target should be projected");

        assert!(target.is_active);
        assert_eq!(target.subscription_count, 1);
        assert_eq!(
            target
                .crawl_policy
                .expect("active target has policy")
                .polling,
            PollingPolicy::Interval {
                interval: interval(3600)
            }
        );
        Ok(())
    }

    #[tokio::test]
    async fn crawl_target_projection_aggregates_multiple_subscriptions() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let one_hour = subscription_with(
            SubscriberId::new("one-hour"),
            "crawl-target-aggregate",
            CrawlPolicy::interval(interval(3600)),
        );
        let ten_minutes = subscription_with(
            SubscriberId::new("ten-minutes"),
            "crawl-target-aggregate",
            CrawlPolicy::interval(interval(600)),
        );
        let manual = subscription_with(
            SubscriberId::new("manual"),
            "crawl-target-aggregate",
            CrawlPolicy::manual(),
        );

        let mut tx = db.begin().await?;
        store_subscription(&mut tx, one_hour.clone()).await?;
        store_subscription(&mut tx, ten_minutes).await?;
        store_subscription(&mut tx, manual).await?;
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
            .load_crawl_target_for_endpoint(&one_hour.feed_url)
            .await?
            .expect("crawl target should be projected");

        assert!(target.is_active);
        assert_eq!(target.subscription_count, 3);
        assert_eq!(
            target
                .crawl_policy
                .expect("active target has policy")
                .polling,
            PollingPolicy::Interval {
                interval: interval(600)
            }
        );
        Ok(())
    }

    #[tokio::test]
    async fn crawl_target_projection_recalculates_after_subscription_change() -> anyhow::Result<()>
    {
        let db = migrated_db().await?;
        let mut subscription = subscription("crawl-target-changed");

        let mut tx = db.begin().await?;
        store_subscription(&mut tx, subscription.clone()).await?;
        tx.commit().await?;

        project_crawl_targets(
            &db,
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribed::new(
                subscription_key(&subscription),
            ))],
        )
        .await?;

        subscription.crawl_policy = CrawlPolicy::interval(interval(300));
        let mut tx = db.begin().await?;
        store_subscription(&mut tx, subscription.clone()).await?;
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
            .load_crawl_target_for_endpoint(&subscription.feed_url)
            .await?
            .expect("crawl target should be projected");

        assert!(target.is_active);
        assert_eq!(target.subscription_count, 1);
        assert_eq!(
            target
                .crawl_policy
                .expect("active target has policy")
                .polling,
            PollingPolicy::Interval {
                interval: interval(300)
            }
        );
        Ok(())
    }

    #[tokio::test]
    async fn crawl_target_projection_inactivates_target_after_last_unsubscribe()
    -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("crawl-target-unsubscribed");

        let mut tx = db.begin().await?;
        store_subscription(&mut tx, subscription.clone()).await?;
        tx.commit().await?;

        project_crawl_targets(
            &db,
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribed::new(
                subscription_key(&subscription),
            ))],
        )
        .await?;

        let mut tx = db.begin().await?;
        tx.delete_feed_subscription(&subscription.subscriber_id, &subscription.feed_url)
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
            .load_crawl_target_for_endpoint(&subscription.feed_url)
            .await?
            .expect("crawl target should be projected");

        assert!(!target.is_active);
        assert_eq!(target.subscription_count, 0);
        assert_eq!(target.crawl_policy, None);
        Ok(())
    }
}
