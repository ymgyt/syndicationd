use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use synd_feed::types::{EntryId, FeedUrl};
use tokio::sync::{Mutex, MutexGuard};

use crate::{
    crawl::{
        blob::{BlobRef, PutBlobCommand},
        job::{
            ClaimCrawlJobCommand, ClaimCrawlJobOutcome, CrawlJob, CrawlJobId, CrawlJobState,
            EnqueueCrawlJobCommand, EnqueueCrawlJobOutcome, FinishCrawlJobCommand,
            FinishCrawlJobOutcome,
        },
        result::{CrawlResultRef, CrawlState, RecordCrawlResultCommand, UpsertCrawlStateCommand},
        schedule::{CrawlScheduleCandidate, UpsertCrawlScheduleCommand},
        target_list::{CrawlTarget, FeedEndpointSubscription, FeedEndpointSubscriptionSet},
    },
    db::{
        BlobStoreTx, CommitTx, CrawlCompletionTx, CrawlJobQueueTx, CrawlScheduleTx, CrawlTargetTx,
        EntryProjectionTx, FeedProjectionTx, FeedRegistryDb, SubscriptionTx, TimelineTx,
    },
    entry::{EntryChanges, EntrySet},
    error::{RegistryDbError, RegistryDbResult},
    event::{
        Event, EventCursor, EventCursorPos, EventInterests, EventType, JournalAppendTx, JournalTx,
        JournaledEvent, ProcessorId,
    },
    feed::{FeedSource, UpsertFeedCommand, UpsertFeedOutcome},
    query::{Subscriptions, SubscriptionsQuery, TimelineItemsPage, TimelineItemsQuery},
    subscription::{FeedSubscriptionAttrs, SubscriberId, Subscription, SubscriptionKey},
    timeline::{TimelineCatchup, TimelineKey},
};

/// In-memory registry adapter for orchestration tests that should not exercise SQL semantics.
#[derive(Debug, Clone, Default)]
pub struct InMemoryFeedRegistryDb {
    state: Arc<Mutex<InMemoryState>>,
}

/// Snapshot transaction handle for `InMemoryFeedRegistryDb`.
#[derive(Debug)]
pub struct InMemoryRegistryTx<'a> {
    guard: MutexGuard<'a, InMemoryState>,
    state: InMemoryState,
}

#[derive(Debug, Clone, Default)]
struct InMemoryState {
    journal: Vec<InMemoryJournalEntry>,
    cursors: HashMap<ProcessorId, i64>,
    subscriptions: HashMap<SubscriptionKeyParts, Subscription>,
    crawl_targets: HashMap<String, CrawlTarget>,
    crawl_states: HashMap<String, CrawlState>,
    blobs: HashMap<i64, Vec<u8>>,
    jobs: Vec<CrawlJob>,
    next_blob_pk: i64,
    next_result_pk: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SubscriptionKeyParts {
    subscriber_id: String,
    feed_url: String,
}

impl SubscriptionKeyParts {
    fn new(subscriber_id: &SubscriberId, feed_url: &FeedUrl) -> Self {
        Self {
            subscriber_id: subscriber_id.as_str().to_owned(),
            feed_url: feed_url.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
struct InMemoryJournalEntry {
    position: i64,
    event_type: EventType,
    event: Event,
    occurred_at: DateTime<Utc>,
}

impl InMemoryFeedRegistryDb {
    pub fn new() -> Self {
        Self::default()
    }
}

impl FeedRegistryDb for InMemoryFeedRegistryDb {
    type Tx<'a> = InMemoryRegistryTx<'a>;

    async fn begin(&self) -> Result<Self::Tx<'_>, RegistryDbError> {
        let guard = self.state.lock().await;
        let state = guard.clone();
        Ok(InMemoryRegistryTx { guard, state })
    }
}

impl CommitTx for InMemoryRegistryTx<'_> {
    async fn commit(mut self) -> RegistryDbResult<()> {
        *self.guard = std::mem::take(&mut self.state);
        Ok(())
    }
}

impl JournalAppendTx for InMemoryRegistryTx<'_> {
    async fn append_event(
        &mut self,
        event: Event,
        occurred_at: DateTime<Utc>,
    ) -> RegistryDbResult<EventType> {
        let event_type = event.event_type();
        let state = &mut self.state;
        let position = i64::try_from(state.journal.len())
            .map_err(|_| RegistryDbError::internal_message("event journal position overflow"))?
            .saturating_add(1);
        state.journal.push(InMemoryJournalEntry {
            position,
            event_type,
            event,
            occurred_at,
        });
        Ok(event_type)
    }
}

impl JournalTx for InMemoryRegistryTx<'_> {
    async fn read_after(
        &mut self,
        cursor: &EventCursor,
        interests: EventInterests,
    ) -> RegistryDbResult<crate::event::EventReadBatch> {
        let position = cursor_position(cursor.position())?;
        let state = &self.state;
        let scanned_position = state
            .journal
            .iter()
            .filter(|entry| entry.position > position)
            .map(|entry| entry.position)
            .max()
            .unwrap_or(position);
        let scanned_cursor = EventCursor::at(
            cursor.processor(),
            EventCursorPos::position(scanned_position.to_string()),
        );
        if interests.types().is_empty() || scanned_position <= position {
            return Ok(crate::event::EventReadBatch::empty(scanned_cursor));
        }

        let events = state
            .journal
            .iter()
            .filter(|entry| {
                entry.position > position
                    && entry.position <= scanned_position
                    && interests.contains(entry.event_type)
            })
            .map(|entry| {
                JournaledEvent::new(
                    EventCursor::at(
                        cursor.processor(),
                        EventCursorPos::position(entry.position.to_string()),
                    ),
                    entry.event.clone(),
                    entry.occurred_at,
                )
            })
            .collect();

        Ok(crate::event::EventReadBatch::new(events, scanned_cursor))
    }

    async fn load_cursor(&mut self, processor: ProcessorId) -> RegistryDbResult<EventCursor> {
        let state = &self.state;
        let Some(position) = state.cursors.get(&processor).copied() else {
            return Ok(EventCursor::initial(processor));
        };
        Ok(EventCursor::at(
            processor,
            EventCursorPos::position(position.to_string()),
        ))
    }

    async fn advance_cursor(&mut self, cursor: &EventCursor) -> RegistryDbResult<()> {
        let position = cursor_position(cursor.position())?;
        let state = &mut self.state;
        let stored = state.cursors.entry(cursor.processor()).or_default();
        *stored = (*stored).max(position);
        Ok(())
    }
}

impl SubscriptionTx for InMemoryRegistryTx<'_> {
    async fn upsert_feed_endpoint(
        &mut self,
        _feed_url: &FeedUrl,
        _now: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        Ok(())
    }

    async fn upsert_feed_subscription(
        &mut self,
        subscription: &SubscriptionKey,
        attrs: FeedSubscriptionAttrs,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        let state = &mut self.state;
        let key = SubscriptionKeyParts::new(&subscription.subscriber_id, &subscription.feed_url);
        let created_at = state
            .subscriptions
            .get(&key)
            .map_or(now, |subscription| subscription.created_at);
        state.subscriptions.insert(
            key,
            Subscription {
                subscriber_id: subscription.subscriber_id.clone(),
                feed_url: subscription.feed_url.clone(),
                requirement: attrs.requirement,
                category: attrs.category,
                crawl_policy: attrs.crawl_policy,
                created_at,
                updated_at: now,
            },
        );
        Ok(())
    }

    async fn delete_feed_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<()> {
        let state = &mut self.state;
        state
            .subscriptions
            .remove(&SubscriptionKeyParts::new(subscriber_id, feed_url));
        Ok(())
    }

    async fn has_feed_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<bool> {
        let state = &self.state;
        Ok(state
            .subscriptions
            .contains_key(&SubscriptionKeyParts::new(subscriber_id, feed_url)))
    }

    async fn list_subscriptions(
        &mut self,
        query: SubscriptionsQuery,
    ) -> RegistryDbResult<Subscriptions> {
        let state = &self.state;
        let mut subscriptions = state
            .subscriptions
            .values()
            .filter(|subscription| subscription.subscriber_id == query.subscriber_id)
            .filter(|subscription| {
                query
                    .after
                    .as_deref()
                    .is_none_or(|after| subscription.feed_url.as_str() > after)
            })
            .cloned()
            .collect::<Vec<_>>();
        subscriptions.sort_by(|a, b| a.feed_url.as_str().cmp(b.feed_url.as_str()));
        let has_next_page = subscriptions.len() > query.first;
        if has_next_page {
            subscriptions.truncate(query.first);
        }
        let end_cursor = subscriptions
            .last()
            .map(|subscription| subscription.feed_url.to_string());
        Ok(Subscriptions::from_subscriptions(
            subscriptions,
            has_next_page,
            end_cursor,
        ))
    }

    async fn load_feed_endpoint_subscriptions(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<FeedEndpointSubscriptionSet> {
        let state = &self.state;
        let mut subscriptions = state
            .subscriptions
            .values()
            .filter(|subscription| subscription.feed_url == *feed_url)
            .map(|subscription| {
                FeedEndpointSubscription::new(
                    SubscriptionKey::new(
                        subscription.subscriber_id.clone(),
                        subscription.feed_url.clone(),
                    ),
                    subscription.crawl_policy,
                )
            })
            .collect::<Vec<_>>();
        subscriptions.sort_by(|a, b| {
            a.subscription
                .subscriber_id
                .as_str()
                .cmp(b.subscription.subscriber_id.as_str())
        });
        Ok(FeedEndpointSubscriptionSet::new(
            feed_url.clone(),
            subscriptions,
        ))
    }
}

impl CrawlTargetTx for InMemoryRegistryTx<'_> {
    async fn upsert_crawl_target(&mut self, target: &CrawlTarget) -> RegistryDbResult<()> {
        let state = &mut self.state;
        state
            .crawl_targets
            .insert(target.feed_url.as_str().to_owned(), target.clone());
        Ok(())
    }

    async fn load_crawl_target_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<CrawlTarget>> {
        let state = &self.state;
        Ok(state.crawl_targets.get(feed_url.as_str()).cloned())
    }
}

impl CrawlScheduleTx for InMemoryRegistryTx<'_> {
    async fn list_candidates(
        &mut self,
        _now: DateTime<Utc>,
        _limit: usize,
    ) -> RegistryDbResult<Vec<CrawlScheduleCandidate>> {
        Ok(Vec::new())
    }

    async fn upsert_schedule(
        &mut self,
        _command: UpsertCrawlScheduleCommand,
    ) -> RegistryDbResult<()> {
        Ok(())
    }
}

impl CrawlJobQueueTx for InMemoryRegistryTx<'_> {
    async fn enqueue_job(
        &mut self,
        command: EnqueueCrawlJobCommand,
    ) -> RegistryDbResult<EnqueueCrawlJobOutcome> {
        let state = &mut self.state;
        if state.jobs.iter().any(|job| {
            job.feed_url == command.feed_url
                && matches!(job.state, CrawlJobState::Pending | CrawlJobState::Running)
        }) {
            return Ok(EnqueueCrawlJobOutcome::AlreadyActive);
        }

        let job = CrawlJob::new(
            CrawlJobId::generate(),
            command.feed_url,
            CrawlJobState::Pending,
            command.trigger,
            command.queue,
            command.priority,
            command.run_after,
            command.enqueued_at,
            command.enqueued_at,
        );
        state.jobs.push(job.clone());
        Ok(EnqueueCrawlJobOutcome::Enqueued(job))
    }

    async fn claim_job(
        &mut self,
        command: ClaimCrawlJobCommand,
    ) -> RegistryDbResult<ClaimCrawlJobOutcome> {
        let state = &mut self.state;
        let Some(job) = state.jobs.iter_mut().find(|job| {
            job.state == CrawlJobState::Pending
                && job.queue == command.queue
                && job.run_after <= command.claimed_at
        }) else {
            return Ok(ClaimCrawlJobOutcome::NoClaimableJob);
        };
        job.state = CrawlJobState::Running;
        job.updated_at = command.claimed_at;
        Ok(ClaimCrawlJobOutcome::Claimed(job.clone()))
    }

    async fn finish_job(
        &mut self,
        command: FinishCrawlJobCommand,
    ) -> RegistryDbResult<FinishCrawlJobOutcome> {
        let state = &mut self.state;
        let Some(job) = state
            .jobs
            .iter_mut()
            .find(|job| job.job_id == command.job_id && job.state == CrawlJobState::Running)
        else {
            return Ok(FinishCrawlJobOutcome::NotRunning);
        };
        job.state = CrawlJobState::Finished;
        job.updated_at = command.finished_at;
        Ok(FinishCrawlJobOutcome::Finished(job.clone()))
    }
}

impl BlobStoreTx for InMemoryRegistryTx<'_> {
    async fn put_blob(&mut self, command: PutBlobCommand) -> RegistryDbResult<BlobRef> {
        let state = &mut self.state;
        state.next_blob_pk = state.next_blob_pk.saturating_add(1);
        let blob = BlobRef::new(state.next_blob_pk);
        state.blobs.insert(blob.pk(), command.bytes);
        Ok(blob)
    }

    async fn load_blob(&mut self, blob: BlobRef) -> RegistryDbResult<Vec<u8>> {
        let state = &self.state;
        state.blobs.get(&blob.pk()).cloned().ok_or_else(|| {
            RegistryDbError::internal_message(format!("blob not found: {}", blob.pk()))
        })
    }
}

impl CrawlCompletionTx for InMemoryRegistryTx<'_> {
    async fn load_crawl_state(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<CrawlState>> {
        let state = &self.state;
        Ok(state.crawl_states.get(feed_url.as_str()).cloned())
    }

    async fn record_crawl_result(
        &mut self,
        _command: RecordCrawlResultCommand,
    ) -> RegistryDbResult<CrawlResultRef> {
        let state = &mut self.state;
        state.next_result_pk = state.next_result_pk.saturating_add(1);
        Ok(CrawlResultRef::new(state.next_result_pk))
    }

    async fn upsert_crawl_state(
        &mut self,
        command: UpsertCrawlStateCommand,
    ) -> RegistryDbResult<()> {
        let state = &mut self.state;
        state.crawl_states.insert(
            command.feed_url.as_str().to_owned(),
            CrawlState {
                feed_url: command.feed_url,
                last: command.last,
                health: command.health,
                conditional: command.conditional,
                timestamps: crate::crawl::result::CrawlStateTimestamps::new(
                    command.updated_at,
                    command.updated_at,
                ),
            },
        );
        Ok(())
    }
}

impl FeedProjectionTx for InMemoryRegistryTx<'_> {
    async fn load_feed_source(
        &mut self,
        _job_id: &CrawlJobId,
    ) -> RegistryDbResult<Option<FeedSource>> {
        Ok(None)
    }

    async fn upsert_feed(
        &mut self,
        _command: UpsertFeedCommand,
    ) -> RegistryDbResult<UpsertFeedOutcome> {
        Ok(UpsertFeedOutcome::Unchanged)
    }
}

impl EntryProjectionTx for InMemoryRegistryTx<'_> {
    async fn load_entry_source(
        &mut self,
        _job_id: &CrawlJobId,
    ) -> RegistryDbResult<Option<FeedSource>> {
        Ok(None)
    }

    async fn load_entries(
        &mut self,
        feed_url: &FeedUrl,
        _entry_ids: &[EntryId],
    ) -> RegistryDbResult<EntrySet> {
        Ok(EntrySet::empty(feed_url.clone()))
    }

    async fn apply_entry_changes(&mut self, _changes: EntryChanges) -> RegistryDbResult<()> {
        Ok(())
    }
}

impl TimelineTx for InMemoryRegistryTx<'_> {
    async fn list_timeline_items(
        &mut self,
        _query: TimelineItemsQuery,
    ) -> RegistryDbResult<TimelineItemsPage> {
        Ok(TimelineItemsPage {
            nodes: Vec::new(),
            has_next_page: false,
            end_cursor: None,
        })
    }

    async fn ensure_default_timeline(
        &mut self,
        _timeline: &TimelineKey,
        _now: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        Ok(())
    }

    async fn catchup_timeline_feed(
        &mut self,
        timeline: &TimelineKey,
        feed_url: &FeedUrl,
        _now: DateTime<Utc>,
    ) -> RegistryDbResult<TimelineCatchup> {
        Ok(TimelineCatchup::new(timeline.clone(), feed_url.clone(), 0))
    }

    async fn apply_entry_discovered(
        &mut self,
        _feed_url: &FeedUrl,
        _entry_id: &EntryId,
        _now: DateTime<Utc>,
    ) -> RegistryDbResult<Vec<TimelineKey>> {
        Ok(Vec::new())
    }

    async fn apply_entry_changed(
        &mut self,
        _feed_url: &FeedUrl,
        _entry_id: &EntryId,
        _now: DateTime<Utc>,
    ) -> RegistryDbResult<Vec<TimelineKey>> {
        Ok(Vec::new())
    }

    async fn apply_feed_unsubscribed(
        &mut self,
        _subscription: &SubscriptionKey,
    ) -> RegistryDbResult<Option<TimelineKey>> {
        Ok(None)
    }
}

fn cursor_position(position: &EventCursorPos) -> RegistryDbResult<i64> {
    match position {
        EventCursorPos::Initial => Ok(0),
        EventCursorPos::Position(position) => {
            let position = position.parse::<i64>().map_err(|err| {
                RegistryDbError::internal_message(format!("invalid event cursor position: {err}"))
            })?;
            if position < 0 {
                return Err(RegistryDbError::internal_message(format!(
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
    use synd_support::time::Clock;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::{
        api::ApiEvent,
        command::SubscribeFeedCommand,
        config::{FeedRegistryConfig, FeedRegistryWorkerConfig},
        crawl::policy::{CrawlPolicy, PollingInterval},
        event::{EventSubmitter, EventWakePublisher, RegistryEvent, SubscribeFeedRequested},
        registry::{FeedRegistry, RegistryService},
    };

    fn test_occurred_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 8, 12, 0, 0).unwrap()
    }

    #[derive(Debug, Clone, Copy)]
    struct TestClock(DateTime<Utc>);

    impl Clock for TestClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn interval(seconds: u64) -> PollingInterval {
        PollingInterval::try_from(Duration::from_secs(seconds)).unwrap()
    }

    fn subscriber_id() -> SubscriberId {
        SubscriberId::new("reader")
    }

    fn feed_url(name: &str) -> FeedUrl {
        let url = format!("https://example.com/{name}.xml");
        FeedUrl::parse(&url).unwrap()
    }

    fn subscribe_command(name: &str, seconds: u64) -> SubscribeFeedCommand {
        SubscribeFeedCommand {
            subscriber_id: subscriber_id(),
            feed_url: feed_url(name),
            requirement: None,
            category: None,
            crawl_policy: CrawlPolicy::interval(interval(seconds)),
        }
    }

    #[tokio::test]
    async fn subscribe_records_request_event() -> anyhow::Result<()> {
        let db = InMemoryFeedRegistryDb::new();
        let config = FeedRegistryConfig::default();
        let event_submitter = EventSubmitter::with_clock(
            db.clone(),
            EventWakePublisher::new(config.event_wake_channel_capacity),
            Arc::new(TestClock(test_occurred_at())),
        );
        let registry = FeedRegistry::new(db.clone(), config, event_submitter);

        registry.subscribe(subscribe_command("event", 3600)).await?;

        let mut tx = db.begin().await?;
        let cursor = tx.load_cursor(ProcessorId::SubscriptionRequest).await?;
        let batch = tx
            .read_after(&cursor, EventInterests::new([SubscribeFeedRequested::TYPE]))
            .await?;
        tx.commit().await?;

        assert_eq!(batch.events().len(), 1);
        assert_eq!(
            batch.events()[0].event().event_type(),
            SubscribeFeedRequested::TYPE
        );
        assert_eq!(batch.events()[0].occurred_at(), test_occurred_at());
        Ok(())
    }

    #[tokio::test]
    async fn runtime_subscribe_projects_subscription_and_api_event() -> anyhow::Result<()> {
        let db = InMemoryFeedRegistryDb::new();
        let ct = CancellationToken::new();
        let config = FeedRegistryConfig {
            workers: FeedRegistryWorkerConfig::with_poll_interval(Duration::from_millis(10)),
            ..FeedRegistryConfig::default()
        };
        let registry_service = RegistryService::start(db, config, ct.clone());
        let (registry, event_workers) = registry_service.into_parts();
        let mut api_events = registry.subscribe_api_events(subscriber_id());

        let output = registry
            .subscribe(subscribe_command("runtime-subscribe", 3600))
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
        assert_eq!(event.subscription.subscriber_id, subscriber_id());
        assert_eq!(event.subscription.feed_url, feed_url("runtime-subscribe"));

        let page = registry
            .list_subscriptions(SubscriptionsQuery {
                subscriber_id: subscriber_id(),
                after: None,
                first: 10,
            })
            .await?;

        assert_eq!(page.subscriptions.len(), 1);
        assert_eq!(
            page.subscriptions[0].feed_url,
            feed_url("runtime-subscribe")
        );

        ct.cancel();
        drop(event_workers);
        Ok(())
    }

    #[tokio::test]
    async fn runtime_second_subscribe_emits_api_subscription_changed() -> anyhow::Result<()> {
        let db = InMemoryFeedRegistryDb::new();
        let ct = CancellationToken::new();
        let config = FeedRegistryConfig {
            workers: FeedRegistryWorkerConfig::with_poll_interval(Duration::from_millis(10)),
            ..FeedRegistryConfig::default()
        };
        let registry_service = RegistryService::start(db, config, ct.clone());
        let (registry, event_workers) = registry_service.into_parts();
        let mut api_events = registry.subscribe_api_events(subscriber_id());

        let first = registry
            .subscribe(subscribe_command("runtime-second-subscribe", 3600))
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
            .subscribe(subscribe_command("runtime-second-subscribe", 600))
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
        assert_eq!(event.subscription.subscriber_id, subscriber_id());
        assert_eq!(
            event.subscription.feed_url,
            feed_url("runtime-second-subscribe")
        );

        ct.cancel();
        drop(event_workers);
        Ok(())
    }
}
