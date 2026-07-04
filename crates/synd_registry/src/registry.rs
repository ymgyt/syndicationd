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
        dispatch::{DispatchQueueReader, DispatchQueueWriter, dispatch_queue},
        scheduler::{driver::SchedDriver, reconciler::CrawlReconciler, tier::TierScheduler},
        target_list::CrawlTargetReconciler,
        worker::CrawlWorkerPool,
    },
    db::{
        BlobStore, CommitTx, CrawlResultStore, CrawlScheduleStore, CrawlTargetStore, EntryStore,
        FeedRegistryDb, FeedStore, SubscriptionStore, TimelineStore,
    },
    entry::EntryProj,
    error::FeedRegistryError,
    event::{
        EventJournal, EventJournalAppend, EventWakePublisher, JournalHandler, JournalWorker,
        PostCommitWorker, Processor, ProjectorAdapter, ReconcilerAdapter, Sink, WorkerHandle,
        WorkerSet, spawn_event_loop,
    },
    feed::FeedProj,
    handler::CommandHandler,
    query::{Subscriptions, SubscriptionsQuery, TimelineItemsPage, TimelineItemsQuery},
    subscription::{SubHandler, SubscriberId},
    timeline::TimelineProj,
};

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
        + CrawlTargetStore
        + FeedStore
        + EntryStore
        + SubscriptionStore
        + TimelineStore
        + EventJournalAppend,
{
    pub fn start(db: S, config: FeedRegistryConfig, ct: CancellationToken) -> (Self, WorkerSet) {
        let builder = FeedRegistry::builder(db.clone(), config);
        let event_dispatch = builder.event_dispatch();
        let workers = spawn_event_workers(
            db,
            &event_dispatch.wake_publisher,
            event_dispatch.api_events.clone(),
            config,
            ct,
            Arc::clone(builder.clock()),
        );
        let registry = builder.build();

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
        + CrawlTargetStore
        + FeedStore
        + EntryStore
        + SubscriptionStore
        + TimelineStore
        + EventJournalAppend,
{
    let ctx = WorkerSpawnCtx::new(db, wake_publisher.clone(), config, ct, clock);
    let (dispatch_queue_writer, dispatch_queue_reader) = ctx.dispatch_queue();

    WorkerSet::new(vec![
        ctx.spawn_crawl_target_reconciler(),
        ctx.spawn_feed_projection(),
        ctx.spawn_entry_projection(),
        ctx.spawn_timeline_projection(),
        ctx.spawn_api_event_projection(),
        ctx.spawn_api_event_publisher(api_events),
        ctx.spawn_crawl_scheduler(dispatch_queue_writer),
        ctx.spawn_crawl_worker_pool(dispatch_queue_reader),
    ])
}

struct WorkerSpawnCtx<S> {
    db: S,
    wake_publisher: EventWakePublisher,
    config: FeedRegistryConfig,
    ct: CancellationToken,
    clock: Arc<dyn Clock>,
}

impl<S> WorkerSpawnCtx<S>
where
    S: FeedRegistryDb,
{
    fn new(
        db: S,
        wake_publisher: EventWakePublisher,
        config: FeedRegistryConfig,
        ct: CancellationToken,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            db,
            wake_publisher,
            config,
            ct,
            clock,
        }
    }

    fn spawn_crawl_target_reconciler(&self) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: CrawlTargetStore + SubscriptionStore + EventJournalAppend,
    {
        self.spawn_journal_worker(
            self.config.workers.crawl_target_reconciler_poll_interval,
            ReconcilerAdapter::new(CrawlTargetReconciler::new()),
        )
    }

    fn spawn_feed_projection(&self) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: BlobStore + CrawlResultStore + FeedStore + EventJournalAppend,
    {
        self.spawn_journal_worker(
            self.config.workers.feed_projection_poll_interval,
            ProjectorAdapter::new(FeedProj::new()),
        )
    }

    fn spawn_entry_projection(&self) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: BlobStore + CrawlResultStore + EntryStore + EventJournalAppend,
    {
        self.spawn_journal_worker(
            self.config.workers.entry_projection_poll_interval,
            ProjectorAdapter::new(EntryProj::new()),
        )
    }

    fn spawn_timeline_projection(&self) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: TimelineStore + EventJournalAppend,
    {
        self.spawn_journal_worker(
            self.config.workers.timeline_projection_poll_interval,
            ProjectorAdapter::new(TimelineProj::new()),
        )
    }

    fn spawn_api_event_projection(&self) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: EventJournalAppend,
    {
        self.spawn_journal_worker(
            self.config.workers.api_event_projection_poll_interval,
            ProjectorAdapter::new(ApiEventProj::new()),
        )
    }

    fn spawn_api_event_publisher(&self, api_events: ApiEventPublisher) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: EventJournalAppend,
    {
        self.spawn_post_commit_worker(
            self.config.workers.api_event_publisher_poll_interval,
            api_events,
        )
    }

    fn dispatch_queue(&self) -> (DispatchQueueWriter, DispatchQueueReader) {
        dispatch_queue(self.config.crawl_worker_pool.max_running_jobs.max(1))
    }

    fn spawn_crawl_scheduler(&self, dispatch_queue_writer: DispatchQueueWriter) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: CrawlScheduleStore + EventJournalAppend,
    {
        let sched_driver = SchedDriver::new(Box::new(TierScheduler::new()), dispatch_queue_writer);
        self.spawn_journal_worker(
            self.config.workers.crawl_scheduler_poll_interval,
            ReconcilerAdapter::new(CrawlReconciler::new(sched_driver)),
        )
    }

    fn spawn_crawl_worker_pool(&self, dispatch_queue_reader: DispatchQueueReader) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>:
            BlobStore + CrawlResultStore + EventJournal + EventJournalAppend + Send,
    {
        let fetcher = Arc::new(FeedService::new(
            self.config.crawl_worker_pool.fetch.user_agent,
            self.config.crawl_worker_pool.fetch.max_body_bytes,
        ));
        let pool = CrawlWorkerPool::new(
            self.db.clone(),
            fetcher,
            self.wake_publisher.clone(),
            dispatch_queue_reader,
            self.config.crawl_worker_pool,
            self.ct.clone(),
            Arc::clone(&self.clock),
        );
        spawn_event_loop(
            pool,
            self.wake_publisher.clone(),
            self.config.workers.crawl_worker_pool_poll_interval,
            self.ct.clone(),
        )
    }

    fn spawn_journal_worker<P>(&self, poll_interval: Duration, processor: P) -> WorkerHandle
    where
        P: JournalHandler<S>,
        for<'tx> S::Tx<'tx>: EventJournalAppend,
    {
        spawn_event_loop(
            JournalWorker::new(self.db.clone(), processor, Arc::clone(&self.clock)),
            self.wake_publisher.clone(),
            poll_interval,
            self.ct.clone(),
        )
    }

    fn spawn_post_commit_worker<P>(&self, poll_interval: Duration, processor: P) -> WorkerHandle
    where
        P: Processor + Sink,
    {
        spawn_event_loop(
            PostCommitWorker::new(self.db.clone(), processor),
            self.wake_publisher.clone(),
            poll_interval,
            self.ct.clone(),
        )
    }
}
