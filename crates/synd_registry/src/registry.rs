use std::{sync::Arc, time::Duration};

use synd_feed::feed::service::FeedService;
use synd_support::time::{Clock, SystemClock};
use tokio_util::sync::CancellationToken;

use crate::{
    api::{ApiEventPublisher, ApiEventSubscriber},
    command::{
        RequestCrawlCommand, RequestCrawlOutput, SubscribeFeedCommand, SubscribeFeedOutput,
        UnsubscribeFeedCommand, UnsubscribeFeedOutput,
    },
    config::FeedRegistryConfig,
    crawl::{
        dispatch::{DispatchQueueReader, DispatchQueueWriter, dispatch_queue},
        dispatcher::CrawlDispatcher,
        request::CrawlRequestHandler,
        schedule::CrawlScheduleProj,
        target_list::CrawlTargetProj,
        worker::CrawlWorkerPool,
    },
    db::{
        BlobStore, CommitTx, CrawlResultStore, CrawlScheduleStore, CrawlTargetStore, EntryStore,
        FeedRegistryDb, FeedStore, SubscriptionStore, TimelineStore,
    },
    entry::EntryProj,
    error::FeedRegistryError,
    event::{
        EventJournal, EventJournalAppend, EventLoop, EventWakePublisher, JournalWorker,
        PostCommitWorker, Projector, ReconcilerWorker, Sink, WorkerHandle, WorkerSet,
    },
    feed::FeedProj,
    handler::CommandHandler,
    query::{
        Subscriptions, SubscriptionsQuery, TimelineChangesPage, TimelineChangesQuery,
        TimelineEntriesPage, TimelineEntriesQuery,
    },
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
            crawl_requests: CrawlRequestHandler::new(self.db.clone(), Arc::clone(&self.clock)),
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
    crawl_requests: CrawlRequestHandler<S>,
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
    for<'tx> S::Tx<'tx>: CrawlScheduleStore + EventJournalAppend,
{
    pub async fn request_crawl(
        &self,
        command: RequestCrawlCommand,
    ) -> Result<RequestCrawlOutput, FeedRegistryError> {
        let handled = self.handlers.crawl_requests.handle(command).await?;
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
        let workers = WorkerSpawnCtx::new(
            db,
            event_dispatch.wake_publisher.clone(),
            config,
            ct,
            Arc::clone(builder.clock()),
        )
        .spawn_all(event_dispatch.api_events.clone());
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

    pub async fn list_timeline_entries(
        &self,
        query: TimelineEntriesQuery,
    ) -> Result<TimelineEntriesPage, FeedRegistryError> {
        let mut tx = self.db.begin().await?;
        let page = tx.list_timeline_entries(query).await?;
        tx.commit().await?;
        Ok(page)
    }

    pub async fn list_timeline_changes(
        &self,
        query: TimelineChangesQuery,
    ) -> Result<TimelineChangesPage, FeedRegistryError> {
        let mut tx = self.db.begin().await?;
        let page = tx.list_timeline_changes(query).await?;
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

    fn spawn_all(self, api_events: ApiEventPublisher) -> WorkerSet
    where
        for<'tx> S::Tx<'tx>: BlobStore
            + CrawlResultStore
            + CrawlScheduleStore
            + CrawlTargetStore
            + FeedStore
            + EntryStore
            + SubscriptionStore
            + TimelineStore
            + EventJournal
            + EventJournalAppend
            + Send,
    {
        let (dispatch_queue_writer, dispatch_queue_reader) = self.dispatch_queue();

        WorkerSet::new(vec![
            self.spawn_crawl_target_projection(),
            self.spawn_crawl_schedule_projection(),
            self.spawn_crawl_dispatcher(dispatch_queue_writer),
            self.spawn_crawl_worker_pool(dispatch_queue_reader),
            self.spawn_feed_projection(),
            self.spawn_entry_projection(),
            self.spawn_timeline_projection(),
            self.spawn_api_event_publisher(api_events),
        ])
    }

    fn spawn_crawl_target_projection(&self) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: CrawlTargetStore + SubscriptionStore + EventJournalAppend,
    {
        self.spawn_journal_worker(
            self.config.workers.crawl_target_projection_poll_interval,
            CrawlTargetProj::new(),
        )
    }

    fn spawn_crawl_schedule_projection(&self) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: CrawlScheduleStore + CrawlResultStore + EventJournalAppend,
    {
        self.spawn_journal_worker(
            self.config.workers.crawl_schedule_projection_poll_interval,
            CrawlScheduleProj::new(),
        )
    }

    fn spawn_feed_projection(&self) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: BlobStore + CrawlResultStore + FeedStore + EventJournalAppend,
    {
        self.spawn_journal_worker(
            self.config.workers.feed_projection_poll_interval,
            FeedProj::new(),
        )
    }

    fn spawn_entry_projection(&self) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: BlobStore + CrawlResultStore + EntryStore + EventJournalAppend,
    {
        self.spawn_journal_worker(
            self.config.workers.entry_projection_poll_interval,
            EntryProj::new(),
        )
    }

    fn spawn_timeline_projection(&self) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: TimelineStore + EventJournalAppend,
    {
        self.spawn_journal_worker(
            self.config.workers.timeline_projection_poll_interval,
            TimelineProj::new(),
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

    fn spawn_crawl_dispatcher(&self, dispatch_queue_writer: DispatchQueueWriter) -> WorkerHandle
    where
        for<'tx> S::Tx<'tx>: CrawlScheduleStore + Send,
    {
        let dispatcher = CrawlDispatcher::new(dispatch_queue_writer, self.config.crawl_dispatch);
        EventLoop::new(
            ReconcilerWorker::new(self.db.clone(), dispatcher, Arc::clone(&self.clock)),
            self.wake_publisher.clone(),
            self.config.workers.crawl_dispatcher_poll_interval,
            self.ct.clone(),
        )
        .spawn()
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
        CrawlWorkerPool::new(
            self.db.clone(),
            fetcher,
            self.wake_publisher.clone(),
            dispatch_queue_reader,
            self.config.crawl_worker_pool,
            self.ct.clone(),
            Arc::clone(&self.clock),
        )
        .spawn()
    }

    fn spawn_journal_worker<P>(&self, poll_interval: Duration, projector: P) -> WorkerHandle
    where
        P: Projector<S>,
        for<'tx> S::Tx<'tx>: EventJournalAppend,
    {
        EventLoop::new(
            JournalWorker::new(self.db.clone(), projector, Arc::clone(&self.clock)),
            self.wake_publisher.clone(),
            poll_interval,
            self.ct.clone(),
        )
        .spawn()
    }

    fn spawn_post_commit_worker<P>(&self, poll_interval: Duration, processor: P) -> WorkerHandle
    where
        P: Sink,
    {
        EventLoop::new(
            PostCommitWorker::new(self.db.clone(), processor),
            self.wake_publisher.clone(),
            poll_interval,
            self.ct.clone(),
        )
        .spawn()
    }
}
