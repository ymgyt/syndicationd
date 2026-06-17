use std::{sync::Arc, time::Duration};

use synd_feed::feed::service::FeedService;
use synd_support::time::{Clock, SystemClock};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::{
    api::{ApiEventProj, ApiEventPublisher, ApiEventSubscriber},
    command::{
        SubscribeFeedCommand, SubscribeFeedOutput, UnsubscribeFeedCommand, UnsubscribeFeedOutput,
    },
    config::FeedRegistryConfig,
    crawl::{
        policy::CrawlPolicy, scheduler::CrawlScheduler, target_list::CrawlTargetListProj,
        worker::spawn_crawl_worker_pool,
    },
    db::{
        BlobStore, CommitTx, CrawlJobQueue, CrawlResultStore, CrawlScheduleStore, CrawlTargetStore,
        EntryStore, FeedRegistryDb, FeedStore, SubscriptionStore, TimelineStore,
    },
    entry::EntryProj,
    error::FeedRegistryError,
    event::{
        CursorAdapter, CursorProjector, CursorReconciler, CursorRole, Event, EventJournalAppend,
        EventRecorder, EventWakePublisher, PostCommitAdapter, Processor, Reconciler,
        RecordedEvents, ScanAdapter, Sink, WorkerHandle, WorkerSet, spawn_event_loop,
    },
    feed::FeedProj,
    query::{Subscriptions, SubscriptionsQuery, TimelineItemsPage, TimelineItemsQuery},
    subscription::{
        self, SubscribeOutcome, SubscriberId, SubscriptionCommand, SubscriptionKey,
        SubscriptionState, UnsubscribeOutcome,
    },
    timeline::TimelineProj,
};

/// Owns a registry facade together with the workers required to process its events.
pub struct RegistryService<S> {
    registry: FeedRegistry<S>,
    workers: WorkerSet,
}

impl<S> RegistryService<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: BlobStore
        + CrawlResultStore
        + CrawlScheduleStore
        + CrawlJobQueue
        + CrawlTargetStore
        + FeedStore
        + EntryStore
        + SubscriptionStore
        + TimelineStore
        + EventJournalAppend,
{
    pub fn start(db: S, config: FeedRegistryConfig, ct: CancellationToken) -> Self {
        let api_events = ApiEventPublisher::default();
        let wake_publisher = EventWakePublisher::new(config.event_wake_channel_capacity);
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let registry = FeedRegistry::with_api_events(
            db.clone(),
            config,
            api_events.clone(),
            wake_publisher.clone(),
            Arc::clone(&clock),
        );
        let workers = spawn_event_workers(db, &wake_publisher, api_events, config, ct, clock);

        Self { registry, workers }
    }

    pub fn registry(&self) -> &FeedRegistry<S> {
        &self.registry
    }

    pub fn into_parts(self) -> (FeedRegistry<S>, WorkerSet) {
        (self.registry, self.workers)
    }
}

#[derive(Clone)]
pub struct FeedRegistry<S> {
    db: S,
    config: FeedRegistryConfig,
    api_events: ApiEventPublisher,
    wake_publisher: EventWakePublisher,
    clock: Arc<dyn Clock>,
    subscription_commands: Arc<Mutex<()>>,
}

impl<S> FeedRegistry<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: EventJournalAppend + SubscriptionStore,
{
    pub fn new(db: S, config: FeedRegistryConfig) -> Self {
        Self::with_api_events(
            db,
            config,
            ApiEventPublisher::default(),
            EventWakePublisher::new(config.event_wake_channel_capacity),
            Arc::new(SystemClock),
        )
    }

    pub fn with_api_events(
        db: S,
        config: FeedRegistryConfig,
        api_events: ApiEventPublisher,
        wake_publisher: EventWakePublisher,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            db,
            config,
            api_events,
            wake_publisher,
            clock,
            subscription_commands: Arc::new(Mutex::new(())),
        }
    }

    pub fn subscribe_api_events(&self, subscriber_id: SubscriberId) -> ApiEventSubscriber {
        self.api_events.subscribe(subscriber_id)
    }

    pub fn default_crawl_policy(&self) -> CrawlPolicy {
        self.config.default_crawl_policy
    }

    pub async fn subscribe(
        &self,
        command: SubscribeFeedCommand,
    ) -> Result<SubscribeFeedOutput, FeedRegistryError> {
        let _guard = self.subscription_commands.lock().await;
        let (subscription, attrs) = command.into_parts();
        let mut tx = self.db.begin().await?;
        let state = subscription_state(&mut tx, &subscription).await?;
        let event = subscription::decide(
            SubscriptionCommand::Subscribe {
                attrs: attrs.clone(),
            },
            state,
            subscription.clone(),
        )?;

        tx.upsert_subscription(&subscription, attrs, self.clock.now())
            .await?;
        let event_type = event.event_type();
        self.record_and_commit(tx, event.clone()).await?;

        let outcome_label = match &event {
            Event::FeedSubscribed(_) => "subscribed",
            Event::SubscriptionChanged(_) => "changed",
            event => unreachable!("subscription decider produced unexpected event: {event:?}"),
        };
        info!(
            subscriber_id = subscription.subscriber_id.as_str(),
            feed_url = subscription.feed_url.as_str(),
            outcome = outcome_label,
            event_type = %event_type,
            "registry subscription committed"
        );

        let outcome = match event {
            Event::FeedSubscribed(event) => SubscribeOutcome::Subscribed(event.subscription),
            Event::SubscriptionChanged(event) => SubscribeOutcome::Changed(event.subscription),
            event => unreachable!("subscription decider produced unexpected event: {event:?}"),
        };
        Ok(SubscribeFeedOutput { outcome })
    }

    pub async fn unsubscribe(
        &self,
        command: UnsubscribeFeedCommand,
    ) -> Result<UnsubscribeFeedOutput, FeedRegistryError> {
        let _guard = self.subscription_commands.lock().await;
        let subscription = command.into_subscription();
        let mut tx = self.db.begin().await?;
        let state = subscription_state(&mut tx, &subscription).await?;
        let event = subscription::decide(
            SubscriptionCommand::Unsubscribe,
            state,
            subscription.clone(),
        )?;

        tx.delete_subscription(&subscription.subscriber_id, &subscription.feed_url)
            .await?;
        let event_type = event.event_type();
        self.record_and_commit(tx, event.clone()).await?;

        info!(
            subscriber_id = subscription.subscriber_id.as_str(),
            feed_url = subscription.feed_url.as_str(),
            outcome = "unsubscribed",
            event_type = %event_type,
            "registry subscription committed"
        );

        let outcome = match event {
            Event::FeedUnsubscribed(event) => UnsubscribeOutcome::Unsubscribed(event.subscription),
            event => unreachable!("subscription decider produced unexpected event: {event:?}"),
        };
        Ok(UnsubscribeFeedOutput { outcome })
    }

    async fn record_and_commit(
        &self,
        mut tx: S::Tx<'_>,
        event: Event,
    ) -> Result<(), FeedRegistryError> {
        let mut recorded_events = RecordedEvents::with_capacity(1);
        {
            let mut recorder =
                EventRecorder::new(&mut tx, &mut recorded_events, self.clock.as_ref());
            recorder.record(event).await?;
        }
        tx.commit().await?;
        self.wake_publisher.publish(recorded_events);
        Ok(())
    }
}

async fn subscription_state<Tx>(
    tx: &mut Tx,
    subscription: &SubscriptionKey,
) -> Result<SubscriptionState, FeedRegistryError>
where
    Tx: SubscriptionStore + Send,
{
    if tx
        .has_subscription(&subscription.subscriber_id, &subscription.feed_url)
        .await?
    {
        Ok(SubscriptionState::Subscribed)
    } else {
        Ok(SubscriptionState::NotSubscribed)
    }
}

impl<S> FeedRegistry<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: SubscriptionStore + TimelineStore,
{
    pub async fn list_subscriptions(
        &self,
        query: SubscriptionsQuery,
    ) -> Result<Subscriptions, FeedRegistryError> {
        let mut tx = self.db.begin().await?;
        let page = tx.list_subscriptions(query).await?;
        tx.commit().await?;
        Ok(page)
    }

    pub async fn list_timeline_items(
        &self,
        query: TimelineItemsQuery,
    ) -> Result<TimelineItemsPage, FeedRegistryError> {
        let mut tx = self.db.begin().await?;
        let page = tx.list_timeline_items(query).await?;
        tx.commit().await?;
        Ok(page)
    }
}

fn spawn_event_workers<S>(
    db: S,
    wake_publisher: &EventWakePublisher,
    api_events: ApiEventPublisher,
    config: FeedRegistryConfig,
    ct: CancellationToken,
    clock: Arc<dyn Clock>,
) -> WorkerSet
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: BlobStore
        + CrawlResultStore
        + CrawlScheduleStore
        + CrawlJobQueue
        + CrawlTargetStore
        + FeedStore
        + EntryStore
        + SubscriptionStore
        + TimelineStore
        + EventJournalAppend,
{
    let crawl_target_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.crawl_target_projection_poll_interval,
        ct.clone(),
        CursorReconciler::new(CrawlTargetListProj::new()),
        Arc::clone(&clock),
    );
    let api_event_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.api_event_projection_poll_interval,
        ct.clone(),
        CursorProjector::new(ApiEventProj::new()),
        Arc::clone(&clock),
    );
    let feed_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.feed_projection_poll_interval,
        ct.clone(),
        CursorProjector::new(FeedProj::new()),
        Arc::clone(&clock),
    );
    let entry_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.entry_projection_poll_interval,
        ct.clone(),
        CursorProjector::new(EntryProj::new()),
        Arc::clone(&clock),
    );
    let timeline_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.timeline_projection_poll_interval,
        ct.clone(),
        CursorProjector::new(TimelineProj::new()),
        Arc::clone(&clock),
    );
    let api_event_publisher_worker = spawn_post_commit_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.api_event_publisher_poll_interval,
        ct.clone(),
        api_events,
    );
    let crawl_scheduler_worker = spawn_scan_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.crawl_scheduler_poll_interval,
        ct.clone(),
        CrawlScheduler::new(),
        Arc::clone(&clock),
    );
    let crawl_fetcher = Arc::new(FeedService::new(
        config.crawl_worker_pool.fetch.user_agent,
        config.crawl_worker_pool.fetch.max_body_bytes,
    ));
    let crawl_worker_pool = spawn_crawl_worker_pool(
        db,
        crawl_fetcher,
        wake_publisher.clone(),
        config.workers.crawl_worker_pool_poll_interval,
        config.crawl_worker_pool,
        ct,
        clock,
    );

    WorkerSet::new(vec![
        crawl_target_projection_worker,
        feed_projection_worker,
        entry_projection_worker,
        timeline_projection_worker,
        api_event_projection_worker,
        api_event_publisher_worker,
        crawl_scheduler_worker,
        crawl_worker_pool,
    ])
}

fn spawn_event_worker<S, P>(
    db: S,
    wake_publisher: EventWakePublisher,
    poll_interval: Duration,
    ct: CancellationToken,
    processor: P,
    clock: Arc<dyn Clock>,
) -> WorkerHandle
where
    S: FeedRegistryDb,
    P: CursorRole<S>,
    for<'tx> S::Tx<'tx>: EventJournalAppend,
{
    spawn_event_loop(
        CursorAdapter::new(db, processor, clock),
        wake_publisher,
        poll_interval,
        ct,
    )
}

fn spawn_scan_worker<S, P>(
    db: S,
    wake_publisher: EventWakePublisher,
    poll_interval: Duration,
    ct: CancellationToken,
    processor: P,
    clock: Arc<dyn Clock>,
) -> WorkerHandle
where
    S: FeedRegistryDb,
    P: Reconciler<S>,
    for<'tx> S::Tx<'tx>: EventJournalAppend,
{
    spawn_event_loop(
        ScanAdapter::new(db, processor, clock),
        wake_publisher,
        poll_interval,
        ct,
    )
}

fn spawn_post_commit_worker<S, P>(
    db: S,
    wake_publisher: EventWakePublisher,
    poll_interval: Duration,
    ct: CancellationToken,
    processor: P,
) -> WorkerHandle
where
    S: FeedRegistryDb,
    P: Processor + Sink,
{
    spawn_event_loop(
        PostCommitAdapter::new(db, processor),
        wake_publisher,
        poll_interval,
        ct,
    )
}
