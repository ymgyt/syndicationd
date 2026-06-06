#![allow(clippy::needless_raw_string_hashes)]

use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Row, Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    CommitTx, FeedRegistryDb, FeedSubscriptionAttrs, RegistryDbError, RegistryDbResult, RegistryTx,
    SubscriberId, SubscriptionKey,
    crawl::target_list::{CrawlTarget, FeedEndpointSubscriptionSet},
    event::{
        Event, EventCursor, EventCursorPos, EventEncoding, EventInterests, EventReadBatch,
        JournalTx, JournaledEvent, ProcessorId,
    },
    query::{Subscriptions, SubscriptionsQuery},
};

use self::{
    crawl_target::CrawlTargetTable, feed_endpoint::FeedEndpointTable,
    feed_endpoint_subscription::FeedEndpointSubscriptionTable,
};
use super::SqliteDatabase;

mod codec;
mod crawl_target;
mod feed_endpoint;
mod feed_endpoint_subscription;

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

impl RegistryTx for SqliteRegistryTx<'_> {
    async fn upsert_feed_endpoint(
        &mut self,
        feed_url: &FeedUrl,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        FeedEndpointTable::new(&mut self.tx)
            .upsert(feed_url, now, now)
            .await?;
        Ok(())
    }

    async fn upsert_feed_subscription(
        &mut self,
        subscription: &SubscriptionKey,
        attrs: FeedSubscriptionAttrs,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        FeedEndpointSubscriptionTable::new(&mut self.tx)
            .upsert(subscription, attrs, now)
            .await
    }

    async fn delete_feed_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<()> {
        FeedEndpointSubscriptionTable::new(&mut self.tx)
            .delete(subscriber_id, feed_url)
            .await
    }

    async fn has_feed_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<bool> {
        FeedEndpointSubscriptionTable::new(&mut self.tx)
            .contains(subscriber_id, feed_url)
            .await
    }

    async fn list_subscriptions(
        &mut self,
        query: SubscriptionsQuery,
    ) -> RegistryDbResult<Subscriptions> {
        FeedEndpointSubscriptionTable::new(&mut self.tx)
            .list(query)
            .await
    }

    async fn load_feed_endpoint_subscriptions(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<FeedEndpointSubscriptionSet> {
        FeedEndpointSubscriptionTable::new(&mut self.tx)
            .load_for_endpoint(feed_url)
            .await
    }

    async fn upsert_crawl_target(&mut self, target: &CrawlTarget) -> RegistryDbResult<()> {
        CrawlTargetTable::new(&mut self.tx).upsert(target).await
    }

    async fn load_crawl_target_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<CrawlTarget>> {
        CrawlTargetTable::new(&mut self.tx)
            .load_for_endpoint(feed_url)
            .await
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
        FeedRegistry, FeedRegistryConfig, RegistryService, SubscribeFeedCommand, Subscription,
        SubscriptionKey,
        crawl::{
            policy::{CrawlPolicy, PollingInterval, PollingPolicy},
            target_list::{CrawlTargetListInput, CrawlTargetListProj, CrawlTargetState},
        },
        event::{
            ApiEvent, ConsumeContext, Consumer, CrawlEvent, CrawlEventKind, EventInterests,
            EventSubmitter, EventWakePublisher, FeedSubscribedEvent, FeedUnsubscribedEvent,
            ProcessorId, RequestEventKind, SubEvent, SubEventKind, SubscriptionChangedEvent,
            SubscriptionLifecycle,
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
        let feed_url = subscription.feed_url.clone();
        let key = subscription_key(&subscription);
        let attrs = FeedSubscriptionAttrs {
            requirement: subscription.requirement,
            category: subscription.category,
            crawl_policy: subscription.crawl_policy,
        };
        tx.upsert_feed_endpoint(&feed_url, subscription.created_at)
            .await?;
        tx.upsert_feed_subscription(&key, attrs, subscription.created_at)
            .await
    }

    fn subscribed_event(path: &str) -> Event {
        Event::Sub(SubEvent::FeedSubscribed(FeedSubscribedEvent::new(
            subscription_key(&subscription(path)),
        )))
    }

    fn changed_event(path: &str) -> Event {
        Event::Sub(SubEvent::SubscriptionChanged(
            SubscriptionChangedEvent::new(subscription_key(&subscription(path))),
        ))
    }

    fn subscription_lifecycle_interests() -> EventInterests {
        EventInterests::new([
            SubEventKind::FeedSubscribed.into(),
            SubEventKind::SubscriptionChanged.into(),
            SubEventKind::FeedUnsubscribed.into(),
        ])
    }

    fn crawl_target_interests() -> EventInterests {
        EventInterests::new([
            CrawlEventKind::TargetActivated.into(),
            CrawlEventKind::TargetPolicyChanged.into(),
            CrawlEventKind::TargetDeactivated.into(),
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

    async fn read_crawl_target_events(
        db: &SqliteFeedRegistryDb,
    ) -> anyhow::Result<Vec<CrawlEvent>> {
        let mut tx = db.begin().await?;
        let cursor = tx.load_cursor(ProcessorId::CrawlTargetProjection).await?;
        let batch = tx.read_after(&cursor, crawl_target_interests()).await?;
        tx.commit().await?;

        let events = batch
            .into_events()
            .into_iter()
            .map(|journaled| match journaled.into_event() {
                Event::Crawl(event) => Ok(event),
                event => anyhow::bail!("unexpected event: {event:?}"),
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(events)
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
    async fn runtime_second_subscribe_emits_api_subscription_changed() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let ct = CancellationToken::new();
        let config = FeedRegistryConfig {
            event_worker_poll_interval: Duration::from_millis(10),
            ..FeedRegistryConfig::default()
        };
        let registry_service = RegistryService::start(db.clone(), config, ct.clone());
        let (registry, event_workers) = registry_service.into_parts();
        let subscriber_id = subscriber_id();
        let feed_url = feed_url("runtime-second-subscribe");
        let mut api_events = registry.subscribe_api_events(subscriber_id.clone());

        let first = registry
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
        assert_eq!(event.request_id, first.request_id);

        let second = registry
            .subscribe(SubscribeFeedCommand {
                subscriber_id: subscriber_id.clone(),
                feed_url: feed_url.clone(),
                requirement: None,
                category: None,
                crawl_policy: CrawlPolicy::interval(interval(600)),
            })
            .await?;

        let api_event = tokio::time::timeout(Duration::from_secs(2), api_events.recv()).await?;
        let api_event = match api_event {
            Ok(event) => event,
            Err(err) => anyhow::bail!("api event receive failed: {err:?}"),
        };
        let ApiEvent::FeedSubscriptionChanged(event) = api_event else {
            anyhow::bail!("unexpected api event: {api_event:?}");
        };
        assert_eq!(event.request_id, second.request_id);
        assert_eq!(event.subscription.subscriber_id, subscriber_id);
        assert_eq!(event.subscription.feed_url, feed_url);

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
            .load_feed_endpoint_subscriptions(&subscription.feed_url)
            .await?;

        assert_eq!(page.subscriptions, vec![subscription.clone()]);
        assert_eq!(endpoint_subscriptions.feed_url, subscription.feed_url);
        assert_eq!(endpoint_subscriptions.subscriptions.len(), 1);
        assert_eq!(
            endpoint_subscriptions.subscriptions[0].subscription,
            subscription_key(&subscription)
        );
        assert_eq!(
            endpoint_subscriptions.subscriptions[0].crawl_policy,
            subscription.crawl_policy
        );
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
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribedEvent::new(
                subscription_key(&subscription),
            ))],
        )
        .await?;

        let mut tx = db.begin().await?;
        let target = tx
            .load_crawl_target_for_endpoint(&subscription.feed_url)
            .await?
            .expect("crawl target should be projected");

        let CrawlTargetState::Active {
            subscription_count,
            effective_policy,
        } = target.state
        else {
            anyhow::bail!("crawl target should be active");
        };
        assert_eq!(subscription_count.get(), 1);
        assert_eq!(
            effective_policy.polling,
            PollingPolicy::Interval {
                interval: interval(3600)
            }
        );
        Ok(())
    }

    #[tokio::test]
    async fn crawl_target_projection_emits_target_activated_event() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("crawl-target-activated-event");

        let mut tx = db.begin().await?;
        store_subscription(&mut tx, subscription.clone()).await?;
        tx.commit().await?;

        project_crawl_targets(
            &db,
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribedEvent::new(
                subscription_key(&subscription),
            ))],
        )
        .await?;

        let events = read_crawl_target_events(&db).await?;
        assert_eq!(events.len(), 1);
        let CrawlEvent::TargetActivated(event) = &events[0] else {
            anyhow::bail!("unexpected crawl event: {:?}", events[0]);
        };
        assert_eq!(event.feed_url, subscription.feed_url);
        assert_eq!(event.policy, subscription.crawl_policy);
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
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribedEvent::new(
                subscription_key(&one_hour),
            ))],
        )
        .await?;

        let mut tx = db.begin().await?;
        let target = tx
            .load_crawl_target_for_endpoint(&one_hour.feed_url)
            .await?
            .expect("crawl target should be projected");

        let CrawlTargetState::Active {
            subscription_count,
            effective_policy,
        } = target.state
        else {
            anyhow::bail!("crawl target should be active");
        };
        assert_eq!(subscription_count.get(), 3);
        assert_eq!(
            effective_policy.polling,
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
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribedEvent::new(
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
            vec![SubscriptionLifecycle::Changed(
                SubscriptionChangedEvent::new(subscription_key(&subscription)),
            )],
        )
        .await?;

        let mut tx = db.begin().await?;
        let target = tx
            .load_crawl_target_for_endpoint(&subscription.feed_url)
            .await?
            .expect("crawl target should be projected");

        let CrawlTargetState::Active {
            subscription_count,
            effective_policy,
        } = target.state
        else {
            anyhow::bail!("crawl target should be active");
        };
        assert_eq!(subscription_count.get(), 1);
        assert_eq!(
            effective_policy.polling,
            PollingPolicy::Interval {
                interval: interval(300)
            }
        );
        Ok(())
    }

    #[tokio::test]
    async fn crawl_target_projection_emits_target_policy_changed_event() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let mut subscription = subscription("crawl-target-policy-changed-event");

        let mut tx = db.begin().await?;
        store_subscription(&mut tx, subscription.clone()).await?;
        tx.commit().await?;

        project_crawl_targets(
            &db,
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribedEvent::new(
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
            vec![SubscriptionLifecycle::Changed(
                SubscriptionChangedEvent::new(subscription_key(&subscription)),
            )],
        )
        .await?;

        let events = read_crawl_target_events(&db).await?;
        assert_eq!(events.len(), 2);
        let CrawlEvent::TargetPolicyChanged(event) = &events[1] else {
            anyhow::bail!("unexpected crawl event: {:?}", events[1]);
        };
        assert_eq!(event.feed_url, subscription.feed_url);
        assert_eq!(event.policy, subscription.crawl_policy);
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
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribedEvent::new(
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
            vec![SubscriptionLifecycle::Unsubscribed(
                FeedUnsubscribedEvent::new(subscription_key(&subscription)),
            )],
        )
        .await?;

        let mut tx = db.begin().await?;
        let target = tx
            .load_crawl_target_for_endpoint(&subscription.feed_url)
            .await?
            .expect("crawl target should be projected");

        assert_eq!(target.state, CrawlTargetState::Inactive);
        Ok(())
    }

    #[tokio::test]
    async fn crawl_target_projection_emits_target_deactivated_event() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("crawl-target-deactivated-event");

        let mut tx = db.begin().await?;
        store_subscription(&mut tx, subscription.clone()).await?;
        tx.commit().await?;

        project_crawl_targets(
            &db,
            vec![SubscriptionLifecycle::Subscribed(FeedSubscribedEvent::new(
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
            vec![SubscriptionLifecycle::Unsubscribed(
                FeedUnsubscribedEvent::new(subscription_key(&subscription)),
            )],
        )
        .await?;

        let events = read_crawl_target_events(&db).await?;
        assert_eq!(events.len(), 2);
        let CrawlEvent::TargetDeactivated(event) = &events[1] else {
            anyhow::bail!("unexpected crawl event: {:?}", events[1]);
        };
        assert_eq!(event.feed_url, subscription.feed_url);
        Ok(())
    }
}
