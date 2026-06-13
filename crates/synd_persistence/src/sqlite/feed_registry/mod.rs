#![allow(clippy::needless_raw_string_hashes)]

use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Row, Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    CommitTx, CrawlJobQueueTx, CrawlScheduleTx, FeedRegistryDb, FeedSubscriptionAttrs,
    RegistryDbError, RegistryDbResult, RegistryTx, SubscriberId, SubscriptionKey,
    crawl::{
        job::{
            ClaimCrawlJobCommand, ClaimCrawlJobOutcome, EnqueueCrawlJobCommand,
            EnqueueCrawlJobOutcome, FinishCrawlJobCommand, FinishCrawlJobOutcome,
        },
        schedule::{CrawlScheduleCandidate, UpsertCrawlScheduleCommand},
        target_list::{CrawlTarget, FeedEndpointSubscriptionSet},
    },
    event::{
        Event, EventCursor, EventCursorPos, EventEncoding, EventInterests, EventReadBatch,
        JournalTx, JournaledEvent, ProcessorId,
    },
    query::{Subscriptions, SubscriptionsQuery, TimelineItemsPage, TimelineItemsQuery},
};

use self::{
    crawl_job::CrawlJobTable, crawl_schedule::CrawlScheduleTable, crawl_target::CrawlTargetTable,
    feed_endpoint::FeedEndpointTable, feed_endpoint_subscription::FeedEndpointSubscriptionTable,
    timeline::TimelineTable,
};
use super::SqliteDatabase;

mod blob;
mod codec;
mod crawl_job;
mod crawl_result;
mod crawl_schedule;
mod crawl_target;
mod entry;
mod feed;
mod feed_endpoint;
mod feed_endpoint_subscription;
mod timeline;

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

    async fn list_timeline_items(
        &mut self,
        query: TimelineItemsQuery,
    ) -> RegistryDbResult<TimelineItemsPage> {
        TimelineTable::new(&mut self.tx).list_items(query).await
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

impl CrawlScheduleTx for SqliteRegistryTx<'_> {
    async fn list_candidates(
        &mut self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> RegistryDbResult<Vec<CrawlScheduleCandidate>> {
        CrawlScheduleTable::new(&mut self.tx)
            .list_candidates(now, limit)
            .await
    }

    async fn upsert_schedule(
        &mut self,
        schedule: UpsertCrawlScheduleCommand,
    ) -> RegistryDbResult<()> {
        CrawlScheduleTable::new(&mut self.tx).upsert(schedule).await
    }
}

impl CrawlJobQueueTx for SqliteRegistryTx<'_> {
    async fn enqueue_job(
        &mut self,
        job: EnqueueCrawlJobCommand,
    ) -> RegistryDbResult<EnqueueCrawlJobOutcome> {
        CrawlJobTable::new(&mut self.tx).enqueue(job).await
    }

    async fn claim_job(
        &mut self,
        command: ClaimCrawlJobCommand,
    ) -> RegistryDbResult<ClaimCrawlJobOutcome> {
        CrawlJobTable::new(&mut self.tx).claim(command).await
    }

    async fn finish_job(
        &mut self,
        command: FinishCrawlJobCommand,
    ) -> RegistryDbResult<FinishCrawlJobOutcome> {
        CrawlJobTable::new(&mut self.tx).finish(command).await
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

    use chrono::{DateTime, TimeZone, Utc};
    use synd_feed::feed::service::{
        FeedFetchFailure, FeedFetchFailureKind, FeedFetchOutcome, FeedHttpResponse, FeedHttpStatus,
        FeedResponseBody, FeedResponseHeaders, FeedService, FetchedFeed,
    };
    use synd_feed::types::{EntryId, FeedUrl};
    use synd_registry::{
        BlobStoreTx, CrawlCompletionTx, FeedRegistry, FeedRegistryConfig, FeedRegistryWorkerConfig,
        RegistryService, SubscribeFeedCommand, Subscription, SubscriptionKey,
        crawl::completion::CrawlCompletionRecorder,
        crawl::{
            blob::PutBlobCommand,
            job::{
                ClaimCrawlJobCommand, ClaimCrawlJobOutcome, CrawlJobQueueLane, CrawlJobState,
                CrawlJobTrigger, EnqueueCrawlJobCommand, EnqueueCrawlJobOutcome,
            },
            policy::{CrawlPolicy, PollingInterval, PollingPolicy},
            result::CrawlStateErrorKind,
            schedule::UpsertCrawlScheduleCommand,
            target_list::{CrawlTargetListInput, CrawlTargetListProj, CrawlTargetState},
        },
        entry::{EntryProj, EntryProjectionInput},
        event::{
            ApiEvent, ConsumeContext, Consumer, CrawlEvent, CrawlEventKind, CrawlJobFinishedEvent,
            EntryChangedEvent, EntryDiscoveredEvent, EntryEventKind, Event, EventInterests,
            EventSubmitter, EventWakePublisher, FeedChangedEvent, FeedDiscoveredEvent,
            FeedEventKind, FeedSubscribedEvent, FeedUnsubscribedEvent, InputBatch, ProcessorId,
            RecordedEvents, RequestEventKind, SubEvent, SubEventKind, SubscriptionChangedEvent,
            SubscriptionLifecycle, TimelineEvent, TimelineEventKind, WorkerId,
        },
        feed::{FeedProj, FeedProjectionInput},
        timeline::{TimelineProj, TimelineProjectionInput},
    };
    use tokio_util::sync::CancellationToken;

    use super::*;

    async fn migrated_db() -> Result<SqliteFeedRegistryDb, RegistryDbError> {
        let db = SqliteDatabase::in_memory().await?;
        db.migrate().await?;
        Ok(SqliteFeedRegistryDb::new(db))
    }

    #[tokio::test]
    async fn blob_store_deduplicates_by_uncompressed_digest() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let created_at = Utc.with_ymd_and_hms(2026, 6, 7, 12, 0, 0).unwrap();
        let mut tx = db.begin().await?;

        let first = tx
            .put_blob(PutBlobCommand::new(b"same payload".to_vec(), created_at))
            .await?;
        let second = tx
            .put_blob(PutBlobCommand::new(b"same payload".to_vec(), created_at))
            .await?;

        assert_eq!(first, second);
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) AS count,
                digest_algo,
                compression_algo,
                uncompressed_len
            FROM blob
            "#,
        )
        .fetch_one(&mut *tx.tx)
        .await?;

        assert_eq!(row.try_get::<i64, _>("count")?, 1);
        assert_eq!(row.try_get::<String, _>("digest_algo")?, "sha256");
        assert_eq!(row.try_get::<String, _>("compression_algo")?, "zstd");
        assert_eq!(row.try_get::<i64, _>("uncompressed_len")?, 12);
        assert_eq!(tx.load_blob(first).await?, b"same payload");
        tx.commit().await?;
        Ok(())
    }

    #[tokio::test]
    async fn crawl_completion_records_result_state_and_finished_event() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let feed_url = feed_url("crawl-completion");
        let enqueued_at = Utc.with_ymd_and_hms(2026, 6, 7, 12, 0, 0).unwrap();
        let claimed_at = Utc.with_ymd_and_hms(2026, 6, 7, 12, 1, 0).unwrap();
        let finished_at = Utc.with_ymd_and_hms(2026, 6, 7, 12, 2, 0).unwrap();

        let mut tx = db.begin().await?;
        tx.upsert_feed_endpoint(&feed_url, enqueued_at).await?;
        let enqueue = tx
            .enqueue_job(EnqueueCrawlJobCommand::new(
                feed_url.clone(),
                CrawlJobTrigger::TargetChanged,
                CrawlJobQueueLane::Default,
                0,
                enqueued_at,
                enqueued_at,
            ))
            .await?;
        assert!(matches!(enqueue, EnqueueCrawlJobOutcome::Enqueued(_)));
        tx.commit().await?;

        let mut tx = db.begin().await?;
        let job = match tx
            .claim_job(ClaimCrawlJobCommand::new(
                CrawlJobQueueLane::Default,
                claimed_at,
            ))
            .await?
        {
            ClaimCrawlJobOutcome::Claimed(job) => job,
            ClaimCrawlJobOutcome::NoClaimableJob => anyhow::bail!("job should be claimable"),
        };
        tx.commit().await?;

        let mut tx = db.begin().await?;
        let mut completion_events = RecordedEvents::empty();
        {
            let mut completion = CrawlCompletionRecorder::new(&mut tx, &mut completion_events);
            completion
                .record(
                    job,
                    FeedFetchOutcome::FetchFailed(FeedFetchFailure {
                        kind: FeedFetchFailureKind::Timeout,
                        message: "deadline exceeded".to_owned(),
                    }),
                    None,
                    finished_at,
                )
                .await?;
        }
        tx.commit().await?;

        assert_eq!(
            completion_events.kinds(),
            &[CrawlEventKind::JobFinished.into()]
        );

        let mut tx = db.begin().await?;
        let state = tx
            .load_crawl_state(&feed_url)
            .await?
            .expect("crawl state should be recorded");
        assert_eq!(state.last.http_status, None);
        assert_eq!(state.health.failure_streak.value(), 1);
        assert_eq!(
            state.last.error.map(|error| error.kind),
            Some(CrawlStateErrorKind::Fetch(FeedFetchFailureKind::Timeout))
        );
        assert_eq!(
            tx.claim_job(ClaimCrawlJobCommand::new(
                CrawlJobQueueLane::Default,
                finished_at,
            ))
            .await?,
            ClaimCrawlJobOutcome::NoClaimableJob
        );
        tx.commit().await?;
        Ok(())
    }

    #[tokio::test]
    async fn feed_projection_records_discovered_unchanged_and_changed() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let feed_url = feed_url("feed-projection");
        let first_body = rss_body("first title");

        let first = record_fetched_crawl(&db, &feed_url, first_body.clone(), 0).await?;
        let recorded = project_feed(&db, first).await?;
        assert_eq!(recorded.kinds(), &[FeedEventKind::Discovered.into()]);

        let second = record_fetched_crawl(&db, &feed_url, first_body, 1).await?;
        let recorded = project_feed(&db, second).await?;
        assert!(recorded.is_empty());

        let third = record_fetched_crawl(&db, &feed_url, rss_body("changed title"), 2).await?;
        let recorded = project_feed(&db, third).await?;
        assert_eq!(recorded.kinds(), &[FeedEventKind::Changed.into()]);

        let mut tx = db.begin().await?;
        let row = sqlx::query(
            r#"
            SELECT current_meta_json
            FROM feed
            "#,
        )
        .fetch_one(&mut *tx.tx)
        .await?;
        let meta_json = row.try_get::<String, _>("current_meta_json")?;
        tx.commit().await?;
        assert!(meta_json.contains("changed title"));
        Ok(())
    }

    #[tokio::test]
    async fn entry_projection_records_discovered_already_seen_and_changed() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let feed_url = feed_url("entry-projection");

        let first = record_fetched_crawl(
            &db,
            &feed_url,
            rss_body_with_entry("first feed", "first entry", "entry-1"),
            0,
        )
        .await?;
        let first_feed_event =
            FeedDiscoveredEvent::new(first.feed_url.clone(), first.job_id.clone());
        let recorded = project_feed(&db, first).await?;
        assert_eq!(recorded.kinds(), &[FeedEventKind::Discovered.into()]);
        let recorded = project_entries(&db, EntryProjectionInput::from(first_feed_event)).await?;
        assert_eq!(recorded.kinds(), &[EntryEventKind::Discovered.into()]);
        let (_, first_source_result_pk) = entry_current_row(&db).await?;

        let second = record_fetched_crawl(
            &db,
            &feed_url,
            rss_body_with_entry("second feed", "first entry", "entry-1"),
            1,
        )
        .await?;
        let second_feed_event =
            FeedChangedEvent::new(second.feed_url.clone(), second.job_id.clone());
        let recorded = project_feed(&db, second).await?;
        assert_eq!(recorded.kinds(), &[FeedEventKind::Changed.into()]);
        let recorded = project_entries(&db, EntryProjectionInput::from(second_feed_event)).await?;
        assert!(recorded.is_empty());
        let (_, second_source_result_pk) = entry_current_row(&db).await?;
        assert_ne!(first_source_result_pk, second_source_result_pk);

        let third = record_fetched_crawl(
            &db,
            &feed_url,
            rss_body_with_entry("third feed", "changed entry", "entry-1"),
            2,
        )
        .await?;
        let third_feed_event = FeedChangedEvent::new(third.feed_url.clone(), third.job_id.clone());
        let recorded = project_feed(&db, third).await?;
        assert_eq!(recorded.kinds(), &[FeedEventKind::Changed.into()]);
        let recorded = project_entries(&db, EntryProjectionInput::from(third_feed_event)).await?;
        assert_eq!(recorded.kinds(), &[EntryEventKind::Changed.into()]);

        let (content_json, third_source_result_pk) = entry_current_row(&db).await?;
        assert!(content_json.contains("changed entry"));
        assert_ne!(second_source_result_pk, third_source_result_pk);
        Ok(())
    }

    #[tokio::test]
    async fn timeline_projection_catches_up_existing_feed_entries_after_subscription()
    -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("timeline-projection");

        let crawl = record_fetched_crawl(
            &db,
            &subscription.feed_url,
            rss_body_with_entry("timeline feed", "timeline entry", "entry-1"),
            0,
        )
        .await?;
        let feed_event = FeedDiscoveredEvent::new(crawl.feed_url.clone(), crawl.job_id.clone());
        let recorded = project_feed(&db, crawl).await?;
        assert_eq!(recorded.kinds(), &[FeedEventKind::Discovered.into()]);
        let recorded = project_entries(&db, EntryProjectionInput::from(feed_event)).await?;
        assert_eq!(recorded.kinds(), &[EntryEventKind::Discovered.into()]);

        store_subscription_in_db(&db, subscription.clone()).await?;

        let subscribed = FeedSubscribedEvent::new(subscription_key(&subscription));
        let recorded =
            project_timeline(&db, TimelineProjectionInput::FeedSubscribed(subscribed)).await?;
        assert_eq!(recorded.kinds(), &[TimelineEventKind::Changed.into()]);

        let mut tx = db.begin().await?;
        let page = tx
            .list_timeline_items(TimelineItemsQuery {
                subscriber_id: subscription.subscriber_id.clone(),
                after: None,
                first: 10,
            })
            .await?;
        tx.commit().await?;

        assert_eq!(page.nodes.len(), 1);
        assert!(!page.has_next_page);
        let node = &page.nodes[0];
        assert_eq!(node.subscription, subscription);
        assert_eq!(node.attrs.title.as_deref(), Some("timeline entry"));
        assert_eq!(node.feed_meta.feed.url(), &feed_url("timeline-projection"));
        Ok(())
    }

    #[tokio::test]
    async fn timeline_projection_does_not_emit_changed_when_catchup_inserts_nothing()
    -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("timeline-idempotent");

        let crawl = record_fetched_crawl(
            &db,
            &subscription.feed_url,
            rss_body_with_entry("timeline feed", "timeline entry", "entry-1"),
            0,
        )
        .await?;
        let feed_event = FeedDiscoveredEvent::new(crawl.feed_url.clone(), crawl.job_id.clone());
        let _ = project_feed(&db, crawl).await?;
        let _ = project_entries(&db, EntryProjectionInput::from(feed_event)).await?;

        store_subscription_in_db(&db, subscription.clone()).await?;

        let subscribed = FeedSubscribedEvent::new(subscription_key(&subscription));
        let _ = project_timeline(
            &db,
            TimelineProjectionInput::FeedSubscribed(subscribed.clone()),
        )
        .await?;
        let recorded =
            project_timeline(&db, TimelineProjectionInput::FeedSubscribed(subscribed)).await?;

        assert!(recorded.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn timeline_projection_adds_discovered_entry_for_active_subscription()
    -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("timeline-entry-discovered");

        let crawl = record_fetched_crawl(
            &db,
            &subscription.feed_url,
            rss_body_with_entry("timeline feed", "timeline entry", "entry-1"),
            0,
        )
        .await?;
        let feed_event = FeedDiscoveredEvent::new(crawl.feed_url.clone(), crawl.job_id.clone());
        let _ = project_feed(&db, crawl.clone()).await?;
        let _ = project_entries(&db, EntryProjectionInput::from(feed_event)).await?;
        let entry_id = current_entry_id(&db).await?;

        store_subscription_in_db(&db, subscription.clone()).await?;

        let recorded = project_timeline(
            &db,
            TimelineProjectionInput::EntryDiscovered(EntryDiscoveredEvent::new(
                subscription.feed_url.clone(),
                entry_id,
                crawl.job_id,
            )),
        )
        .await?;
        assert_eq!(recorded.kinds(), &[TimelineEventKind::Changed.into()]);

        let page = list_timeline_items(&db, subscription.subscriber_id.clone()).await?;
        assert_eq!(page.nodes.len(), 1);
        assert_eq!(page.nodes[0].attrs.title.as_deref(), Some("timeline entry"));
        Ok(())
    }

    #[tokio::test]
    async fn timeline_projection_ignores_discovered_entry_without_subscription()
    -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let feed_url = feed_url("timeline-entry-without-subscription");

        let crawl = record_fetched_crawl(
            &db,
            &feed_url,
            rss_body_with_entry("timeline feed", "timeline entry", "entry-1"),
            0,
        )
        .await?;
        let feed_event = FeedDiscoveredEvent::new(crawl.feed_url.clone(), crawl.job_id.clone());
        let _ = project_feed(&db, crawl.clone()).await?;
        let _ = project_entries(&db, EntryProjectionInput::from(feed_event)).await?;
        let entry_id = current_entry_id(&db).await?;

        let recorded = project_timeline(
            &db,
            TimelineProjectionInput::EntryDiscovered(EntryDiscoveredEvent::new(
                feed_url,
                entry_id,
                crawl.job_id,
            )),
        )
        .await?;

        assert!(recorded.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn timeline_projection_invalidates_changed_entry_payload() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("timeline-entry-payload");

        let first = record_fetched_crawl(
            &db,
            &subscription.feed_url,
            rss_body_with_entry("timeline feed", "old title", "entry-1"),
            0,
        )
        .await?;
        let first_feed_event =
            FeedDiscoveredEvent::new(first.feed_url.clone(), first.job_id.clone());
        let _ = project_feed(&db, first.clone()).await?;
        let _ = project_entries(&db, EntryProjectionInput::from(first_feed_event)).await?;
        let entry_id = current_entry_id(&db).await?;

        store_subscription_in_db(&db, subscription.clone()).await?;
        let _ = project_timeline(
            &db,
            TimelineProjectionInput::EntryDiscovered(EntryDiscoveredEvent::new(
                subscription.feed_url.clone(),
                entry_id.clone(),
                first.job_id,
            )),
        )
        .await?;

        let second = record_fetched_crawl(
            &db,
            &subscription.feed_url,
            rss_body_with_entry("timeline feed", "new title", "entry-1"),
            1,
        )
        .await?;
        let second_feed_event =
            FeedChangedEvent::new(second.feed_url.clone(), second.job_id.clone());
        let _ = project_feed(&db, second.clone()).await?;
        let recorded = project_entries(&db, EntryProjectionInput::from(second_feed_event)).await?;
        assert_eq!(recorded.kinds(), &[EntryEventKind::Changed.into()]);

        let recorded = project_timeline(
            &db,
            TimelineProjectionInput::EntryChanged(EntryChangedEvent::new(
                subscription.feed_url.clone(),
                entry_id,
                second.job_id,
            )),
        )
        .await?;
        assert_eq!(recorded.kinds(), &[TimelineEventKind::Changed.into()]);

        let page = list_timeline_items(&db, subscription.subscriber_id.clone()).await?;
        assert_eq!(page.nodes.len(), 1);
        assert_eq!(page.nodes[0].attrs.title.as_deref(), Some("new title"));
        Ok(())
    }

    #[tokio::test]
    async fn timeline_projection_updates_changed_entry_order() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("timeline-entry-order");
        let older = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        let newer = Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap();

        let first = record_fetched_crawl(
            &db,
            &subscription.feed_url,
            rss_body_with_entry_published_at("timeline feed", "old title", "entry-1", older),
            0,
        )
        .await?;
        let first_feed_event =
            FeedDiscoveredEvent::new(first.feed_url.clone(), first.job_id.clone());
        let _ = project_feed(&db, first.clone()).await?;
        let _ = project_entries(&db, EntryProjectionInput::from(first_feed_event)).await?;
        let entry_id = current_entry_id(&db).await?;

        store_subscription_in_db(&db, subscription.clone()).await?;
        let _ = project_timeline(
            &db,
            TimelineProjectionInput::EntryDiscovered(EntryDiscoveredEvent::new(
                subscription.feed_url.clone(),
                entry_id.clone(),
                first.job_id,
            )),
        )
        .await?;
        let page = list_timeline_items(&db, subscription.subscriber_id.clone()).await?;
        assert_eq!(page.nodes[0].cursor.order_time(), older);

        let second = record_fetched_crawl(
            &db,
            &subscription.feed_url,
            rss_body_with_entry_published_at("timeline feed", "new title", "entry-1", newer),
            1,
        )
        .await?;
        let second_feed_event =
            FeedChangedEvent::new(second.feed_url.clone(), second.job_id.clone());
        let _ = project_feed(&db, second.clone()).await?;
        let _ = project_entries(&db, EntryProjectionInput::from(second_feed_event)).await?;

        let recorded = project_timeline(
            &db,
            TimelineProjectionInput::EntryChanged(EntryChangedEvent::new(
                subscription.feed_url.clone(),
                entry_id,
                second.job_id,
            )),
        )
        .await?;
        assert_eq!(recorded.kinds(), &[TimelineEventKind::Changed.into()]);

        let page = list_timeline_items(&db, subscription.subscriber_id.clone()).await?;
        assert_eq!(page.nodes[0].cursor.order_time(), newer);
        Ok(())
    }

    #[tokio::test]
    async fn timeline_projection_coalesces_entry_invalidations_by_feed() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let subscription = subscription("timeline-entry-coalesce");

        let crawl = record_fetched_crawl(
            &db,
            &subscription.feed_url,
            rss_body_with_entry("timeline feed", "timeline entry", "entry-1"),
            0,
        )
        .await?;
        let feed_event = FeedDiscoveredEvent::new(crawl.feed_url.clone(), crawl.job_id.clone());
        let _ = project_feed(&db, crawl.clone()).await?;
        let _ = project_entries(&db, EntryProjectionInput::from(feed_event)).await?;
        let entry_id = current_entry_id(&db).await?;

        store_subscription_in_db(&db, subscription.clone()).await?;

        let input = TimelineProjectionInput::EntryDiscovered(EntryDiscoveredEvent::new(
            subscription.feed_url.clone(),
            entry_id,
            crawl.job_id,
        ));
        let recorded = project_timeline_batch(&db, vec![input.clone(), input]).await?;
        assert_eq!(recorded.kinds(), &[TimelineEventKind::Changed.into()]);

        let events = read_timeline_events(&db).await?;
        let Some(TimelineEvent::Changed(event)) = events.last() else {
            anyhow::bail!("expected timeline changed event");
        };
        assert_eq!(event.affected_feeds, vec![subscription.feed_url]);
        Ok(())
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

    async fn store_subscription_in_db(
        db: &SqliteFeedRegistryDb,
        subscription: Subscription,
    ) -> anyhow::Result<()> {
        let mut tx = db.begin().await?;
        store_subscription(&mut tx, subscription).await?;
        tx.commit().await?;
        Ok(())
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

    fn timeline_interests() -> EventInterests {
        EventInterests::new([TimelineEventKind::Changed.into()])
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

    async fn read_timeline_events(db: &SqliteFeedRegistryDb) -> anyhow::Result<Vec<TimelineEvent>> {
        let mut tx = db.begin().await?;
        let cursor = tx.load_cursor(ProcessorId::ApiEventProjection).await?;
        let batch = tx.read_after(&cursor, timeline_interests()).await?;
        tx.commit().await?;

        let events = batch
            .into_events()
            .into_iter()
            .map(|journaled| match journaled.into_event() {
                Event::Timeline(event) => Ok(event),
                event => anyhow::bail!("unexpected event: {event:?}"),
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(events)
    }

    async fn record_fetched_crawl(
        db: &SqliteFeedRegistryDb,
        feed_url: &FeedUrl,
        body: Vec<u8>,
        seq: i64,
    ) -> anyhow::Result<CrawlJobFinishedEvent> {
        let enqueued_at = Utc.with_ymd_and_hms(2026, 6, 8, 12, 0, 0).unwrap()
            + chrono::Duration::minutes(seq * 3);
        let claimed_at = enqueued_at + chrono::Duration::minutes(1);
        let finished_at = enqueued_at + chrono::Duration::minutes(2);
        let mut tx = db.begin().await?;
        tx.upsert_feed_endpoint(feed_url, enqueued_at).await?;
        let enqueue = tx
            .enqueue_job(EnqueueCrawlJobCommand::new(
                feed_url.clone(),
                CrawlJobTrigger::TargetChanged,
                CrawlJobQueueLane::Default,
                0,
                enqueued_at,
                enqueued_at,
            ))
            .await?;
        assert!(matches!(enqueue, EnqueueCrawlJobOutcome::Enqueued(_)));

        let job = match tx
            .claim_job(ClaimCrawlJobCommand::new(
                CrawlJobQueueLane::Default,
                claimed_at,
            ))
            .await?
        {
            ClaimCrawlJobOutcome::Claimed(job) => job,
            ClaimCrawlJobOutcome::NoClaimableJob => anyhow::bail!("job should be claimable"),
        };
        let event = CrawlJobFinishedEvent::new(job.job_id.clone(), job.feed_url.clone());
        let outcome = fetched_outcome(feed_url.clone(), body, finished_at)?;
        let mut completion_events = RecordedEvents::empty();
        {
            let mut completion = CrawlCompletionRecorder::new(&mut tx, &mut completion_events);
            completion.record(job, outcome, None, finished_at).await?;
        }
        tx.commit().await?;
        Ok(event)
    }

    async fn project_feed(
        db: &SqliteFeedRegistryDb,
        event: CrawlJobFinishedEvent,
    ) -> anyhow::Result<RecordedEvents> {
        let mut tx = db.begin().await?;
        let mut proj = FeedProj::new();
        let recorded = {
            let mut cx = ConsumeContext::new(&mut tx);
            <FeedProj as Consumer<SqliteFeedRegistryDb>>::consume(
                &mut proj,
                &mut cx,
                FeedProjectionInput::new(event),
            )
            .await?;
            cx.into_recorded()
        };
        tx.commit().await?;
        Ok(recorded)
    }

    async fn project_entries(
        db: &SqliteFeedRegistryDb,
        input: EntryProjectionInput,
    ) -> anyhow::Result<RecordedEvents> {
        let mut tx = db.begin().await?;
        let mut proj = EntryProj::new();
        let recorded = {
            let mut cx = ConsumeContext::new(&mut tx);
            <EntryProj as Consumer<SqliteFeedRegistryDb>>::consume(&mut proj, &mut cx, input)
                .await?;
            cx.into_recorded()
        };
        tx.commit().await?;
        Ok(recorded)
    }

    async fn project_timeline(
        db: &SqliteFeedRegistryDb,
        input: TimelineProjectionInput,
    ) -> anyhow::Result<RecordedEvents> {
        project_timeline_batch(db, vec![input]).await
    }

    async fn project_timeline_batch(
        db: &SqliteFeedRegistryDb,
        inputs: Vec<TimelineProjectionInput>,
    ) -> anyhow::Result<RecordedEvents> {
        let mut tx = db.begin().await?;
        let mut proj = TimelineProj::new();
        let recorded = {
            let mut cx = ConsumeContext::new(&mut tx);
            <TimelineProj as Consumer<SqliteFeedRegistryDb>>::consume_batch(
                &mut proj,
                &mut cx,
                InputBatch::new(inputs),
            )
            .await?;
            cx.into_recorded()
        };
        tx.commit().await?;
        Ok(recorded)
    }

    async fn entry_current_row(db: &SqliteFeedRegistryDb) -> anyhow::Result<(String, i64)> {
        let mut tx = db.begin().await?;
        let row = sqlx::query(
            r#"
            SELECT current_content_json, current_source_result_pk
            FROM entry
            "#,
        )
        .fetch_one(&mut *tx.tx)
        .await?;
        let content_json = row.try_get::<String, _>("current_content_json")?;
        let source_result_pk = row.try_get::<i64, _>("current_source_result_pk")?;
        tx.commit().await?;
        Ok((content_json, source_result_pk))
    }

    async fn current_entry_id(db: &SqliteFeedRegistryDb) -> anyhow::Result<EntryId> {
        let mut tx = db.begin().await?;
        let row = sqlx::query(
            r#"
            SELECT entry_id
            FROM entry
            "#,
        )
        .fetch_one(&mut *tx.tx)
        .await?;
        let entry_id = row.try_get::<String, _>("entry_id")?;
        tx.commit().await?;
        EntryId::parse(entry_id).map_err(Into::into)
    }

    async fn list_timeline_items(
        db: &SqliteFeedRegistryDb,
        subscriber_id: SubscriberId,
    ) -> anyhow::Result<TimelineItemsPage> {
        let mut tx = db.begin().await?;
        let page = tx
            .list_timeline_items(TimelineItemsQuery {
                subscriber_id,
                after: None,
                first: 10,
            })
            .await?;
        tx.commit().await?;
        Ok(page)
    }

    fn fetched_outcome(
        feed_url: FeedUrl,
        body: Vec<u8>,
        fetched_at: DateTime<Utc>,
    ) -> anyhow::Result<FeedFetchOutcome> {
        let feed = FeedService::parse_feed(feed_url.clone(), body.as_slice())?;
        Ok(FeedFetchOutcome::Fetched(Box::new(FetchedFeed {
            body: FeedResponseBody {
                response: FeedHttpResponse {
                    requested_url: feed_url.clone(),
                    response_url: feed_url,
                    status: FeedHttpStatus::new(200),
                    headers: FeedResponseHeaders::default(),
                    fetched_at,
                },
                bytes: body,
            },
            feed,
        })))
    }

    fn rss_body(title: &str) -> Vec<u8> {
        rss_body_with_entry(title, "first entry", "entry-1")
    }

    fn rss_body_with_entry(feed_title: &str, entry_title: &str, entry_guid: &str) -> Vec<u8> {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0">
  <channel>
    <title>{feed_title}</title>
    <link>https://example.com/</link>
    <description>example feed</description>
    <item>
      <title>{entry_title}</title>
      <link>https://example.com/entry/1</link>
      <guid>{entry_guid}</guid>
    </item>
  </channel>
</rss>"#
        )
        .into_bytes()
    }

    fn rss_body_with_entry_published_at(
        feed_title: &str,
        entry_title: &str,
        entry_guid: &str,
        published_at: DateTime<Utc>,
    ) -> Vec<u8> {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0">
  <channel>
    <title>{feed_title}</title>
    <link>https://example.com/</link>
    <description>example feed</description>
    <item>
      <title>{entry_title}</title>
      <link>https://example.com/entry/1</link>
      <guid>{entry_guid}</guid>
      <pubDate>{}</pubDate>
    </item>
  </channel>
</rss>"#,
            published_at.to_rfc2822()
        )
        .into_bytes()
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
            workers: FeedRegistryWorkerConfig::with_poll_interval(Duration::from_millis(10)),
            ..FeedRegistryConfig::default()
        };
        let registry_service = RegistryService::start(db.clone(), config, ct.clone());
        let (registry, event_workers) = registry_service.into_parts();
        assert!(
            event_workers
                .handles()
                .iter()
                .any(|worker| worker.id() == WorkerId::CrawlWorkerPool)
        );
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
            workers: FeedRegistryWorkerConfig::with_poll_interval(Duration::from_millis(10)),
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
    async fn crawl_schedule_candidates_and_job_enqueue_are_persisted() -> anyhow::Result<()> {
        let db = migrated_db().await?;
        let now = Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();
        let subscription = subscription("crawl-schedule-candidate");

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
        let candidates = tx.list_candidates(now, 10).await?;
        assert_eq!(candidates.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(candidate.target.feed_url, subscription.feed_url);
        assert!(candidate.schedule.is_none());
        assert!(candidate.active_job.is_none());

        tx.upsert_schedule(UpsertCrawlScheduleCommand::new(
            subscription.feed_url.clone(),
            candidate.target.target_updated_at,
            Some(now),
            now,
        ))
        .await?;

        let result = tx
            .enqueue_job(EnqueueCrawlJobCommand::new(
                subscription.feed_url.clone(),
                CrawlJobTrigger::TargetChanged,
                CrawlJobQueueLane::Default,
                0,
                now,
                now,
            ))
            .await?;
        let EnqueueCrawlJobOutcome::Enqueued(job) = result else {
            anyhow::bail!("expected job to be enqueued");
        };
        assert_eq!(job.feed_url, subscription.feed_url);
        assert_eq!(job.trigger, CrawlJobTrigger::TargetChanged);

        let result = tx
            .enqueue_job(EnqueueCrawlJobCommand::new(
                subscription.feed_url.clone(),
                CrawlJobTrigger::TargetChanged,
                CrawlJobQueueLane::Default,
                0,
                now,
                now,
            ))
            .await?;
        assert_eq!(result, EnqueueCrawlJobOutcome::AlreadyActive);

        let result = tx
            .claim_job(ClaimCrawlJobCommand::new(CrawlJobQueueLane::Default, now))
            .await?;
        let ClaimCrawlJobOutcome::Claimed(claimed) = result else {
            anyhow::bail!("expected job to be claimed");
        };
        assert_eq!(claimed.feed_url, subscription.feed_url);
        assert_eq!(claimed.state, CrawlJobState::Running);
        assert_eq!(claimed.queue, CrawlJobQueueLane::Default);
        tx.commit().await?;

        let mut tx = db.begin().await?;
        let candidates = tx.list_candidates(now, 10).await?;
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].schedule.is_some());
        assert!(candidates[0].active_job.is_some());
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
