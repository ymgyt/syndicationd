use std::{sync::Arc, time::Duration};

use synd_feed::feed::service::FeedService;
use synd_support::time::{Clock, SystemClock};
use tokio_util::sync::CancellationToken;

use crate::{
    api::{ApiEventProj, ApiEventPublisher, ApiEventSubscriber},
    command::{
        SubscribeFeedCommand, SubscribeFeedOutput, UnsubscribeFeedCommand, UnsubscribeFeedOutput,
    },
    config::FeedRegistryConfig,
    crawl::{
        scheduler::CrawlScheduler, target_list::CrawlTargetListProj,
        worker::spawn_crawl_worker_pool,
    },
    db::{
        BlobStore, CommitTx, CrawlJobQueue, CrawlResultStore, CrawlScheduleStore, CrawlTargetStore,
        EntryStore, FeedRegistryDb, FeedStore, SubscriptionStore, TimelineStore,
    },
    entry::EntryProj,
    error::FeedRegistryError,
    event::{
        CursorAdapter, CursorProjector, CursorReconciler, CursorRole, EventJournalAppend,
        EventWakePublisher, PostCommitAdapter, Processor, Reconciler, ScanAdapter, Sink,
        WorkerHandle, WorkerSet, spawn_event_loop,
    },
    feed::FeedProj,
    handler::CommandHandler,
    query::{Subscriptions, SubscriptionsQuery, TimelineItemsPage, TimelineItemsQuery},
    subscription::{SubHandler, SubscriberId},
    timeline::TimelineProj,
};

/// Channels shared by registry commands and post-commit event workers.
#[derive(Clone)]
struct EventDispatch {
    api_events: ApiEventPublisher,
    wake_publisher: EventWakePublisher,
}

impl EventDispatch {
    fn new(config: FeedRegistryConfig) -> Self {
        Self {
            api_events: ApiEventPublisher::default(),
            wake_publisher: EventWakePublisher::new(config.event_wake_channel_capacity),
        }
    }
}

/// Builds a registry facade with shared dispatch channels and clock wiring.
pub(crate) struct FeedRegistryBuilder<S> {
    db: S,
    config: FeedRegistryConfig,
    event_dispatch: EventDispatch,
    clock: Arc<dyn Clock>,
}

impl<S> FeedRegistryBuilder<S>
where
    S: Clone,
{
    fn new(db: S, config: FeedRegistryConfig) -> Self {
        Self {
            db,
            config,
            event_dispatch: EventDispatch::new(config),
            clock: Arc::new(SystemClock),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    fn event_dispatch(&self) -> &EventDispatch {
        &self.event_dispatch
    }

    fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }

    pub(crate) fn build(self) -> FeedRegistry<S> {
        let handlers = RegistryHandlers {
            subscriptions: SubHandler::new(
                self.db.clone(),
                self.config.default_crawl_policy,
                Arc::clone(&self.clock),
            ),
        };

        FeedRegistry {
            db: self.db,
            handlers,
            event_dispatch: self.event_dispatch,
        }
    }
}

/// Command handlers owned by the registry facade.
#[derive(Clone)]
pub(crate) struct RegistryHandlers<S> {
    subscriptions: SubHandler<S>,
}

/// Facade for registry commands, queries, and API event subscriptions.
#[derive(Clone)]
pub struct FeedRegistry<S> {
    db: S,
    handlers: RegistryHandlers<S>,
    event_dispatch: EventDispatch,
}

impl<S> FeedRegistry<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: EventJournalAppend + SubscriptionStore,
{
    pub fn new(db: S, config: FeedRegistryConfig) -> Self {
        Self::builder(db, config).build()
    }

    pub(crate) fn builder(db: S, config: FeedRegistryConfig) -> FeedRegistryBuilder<S> {
        FeedRegistryBuilder::new(db, config)
    }

    pub fn subscribe_events(&self, subscriber_id: SubscriberId) -> ApiEventSubscriber {
        self.event_dispatch.api_events.subscribe(subscriber_id)
    }

    pub async fn subscribe(
        &self,
        command: SubscribeFeedCommand,
    ) -> Result<SubscribeFeedOutput, FeedRegistryError> {
        let handled = self.handlers.subscriptions.handle(command).await?;
        self.event_dispatch
            .wake_publisher
            .publish(handled.recorded_events);
        Ok(handled.output)
    }

    pub async fn unsubscribe(
        &self,
        command: UnsubscribeFeedCommand,
    ) -> Result<UnsubscribeFeedOutput, FeedRegistryError> {
        let handled = self.handlers.subscriptions.handle(command).await?;
        self.event_dispatch
            .wake_publisher
            .publish(handled.recorded_events);
        Ok(handled.output)
    }
}

impl<S> FeedRegistry<S>
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
    pub fn start(db: S, config: FeedRegistryConfig, ct: CancellationToken) -> (Self, WorkerSet) {
        let registry_builder = FeedRegistry::builder(db.clone(), config);
        let event_dispatch = registry_builder.event_dispatch();
        let workers = spawn_event_workers(
            db,
            &event_dispatch.wake_publisher,
            event_dispatch.api_events.clone(),
            config,
            ct,
            Arc::clone(registry_builder.clock()),
        );
        let registry = registry_builder.build();

        (registry, workers)
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
