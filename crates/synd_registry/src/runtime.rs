use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::{
    FeedRegistry,
    config::FeedRegistryConfig,
    consumers::{ApiEventProj, ApiEventStream, SubRequestWorker},
    crawl::target_list::CrawlTargetListProj,
    db::FeedRegistryDb,
    event::{
        ApiEventPublisher, EventConsumer, EventJournal, EventRuntime, EventWakePublisher,
        EventWakeSubmitter, Worker, WorkerHandle, WorkerSet,
    },
};

pub type RuntimeEventSubmitter<J> = EventWakeSubmitter<EventRuntime<J>>;

pub type RuntimeFeedRegistry<S, J> = FeedRegistry<S, RuntimeEventSubmitter<J>>;

/// Owns the tasks that make a feed registry live.
pub struct FeedRegistryRuntime<S, J>
where
    S: FeedRegistryDb,
    J: EventJournal,
{
    registry: RuntimeFeedRegistry<S, J>,
    event_workers: WorkerSet,
}

impl<S, J> FeedRegistryRuntime<S, J>
where
    S: FeedRegistryDb,
    J: EventJournal,
{
    pub fn start(db: S, journal: J, config: FeedRegistryConfig, ct: CancellationToken) -> Self {
        let api_events = ApiEventPublisher::default();
        let wake_publisher = EventWakePublisher::new(config.event_wake_channel_capacity);
        let event_runtime =
            EventWakeSubmitter::new(EventRuntime::new(journal.clone()), wake_publisher.clone());
        let registry =
            FeedRegistry::with_event_runtime(db.clone(), config, api_events.clone(), event_runtime);
        let event_workers =
            spawn_event_workers(db, journal, wake_publisher, api_events, config, ct);

        Self {
            registry,
            event_workers,
        }
    }

    pub fn registry(&self) -> RuntimeFeedRegistry<S, J> {
        self.registry.clone()
    }

    pub fn event_workers(&self) -> &WorkerSet {
        &self.event_workers
    }

    #[expect(clippy::unused_async)]
    pub async fn reconcile_startup(&self) {
        tracing::debug!("startup feed reconcile is disabled while crawl runtime is redesigned");
    }
}

impl<S, J> Drop for FeedRegistryRuntime<S, J>
where
    S: FeedRegistryDb,
    J: EventJournal,
{
    fn drop(&mut self) {
        self.event_workers.abort();
    }
}

fn spawn_event_workers<S, J>(
    db: S,
    journal: J,
    wake_publisher: EventWakePublisher,
    api_events: ApiEventPublisher,
    config: FeedRegistryConfig,
    ct: CancellationToken,
) -> WorkerSet
where
    S: FeedRegistryDb,
    J: EventJournal,
{
    let spawn_worker = EventWorkerSpawner::new(
        journal,
        wake_publisher,
        config.event_worker_poll_interval,
        ct,
    );

    WorkerSet::new(vec![
        spawn_worker.spawn(SubRequestWorker::new(db)),
        spawn_worker.spawn(CrawlTargetListProj::new()),
        spawn_worker.spawn(ApiEventProj::new()),
        spawn_worker.spawn(ApiEventStream::new(api_events)),
    ])
}

struct EventWorkerSpawner<J> {
    journal: J,
    wake_publisher: EventWakePublisher,
    poll_interval: Duration,
    ct: CancellationToken,
}

impl<J> EventWorkerSpawner<J>
where
    J: EventJournal,
{
    fn new(
        journal: J,
        wake_publisher: EventWakePublisher,
        poll_interval: Duration,
        ct: CancellationToken,
    ) -> Self {
        Self {
            journal,
            wake_publisher,
            poll_interval,
            ct,
        }
    }

    fn spawn<C>(&self, consumer: C) -> WorkerHandle
    where
        C: EventConsumer,
    {
        Worker::new(
            self.journal.clone(),
            consumer,
            self.wake_publisher.clone(),
            self.wake_publisher.subscribe(),
            self.poll_interval,
        )
        .spawn(self.ct.clone())
    }
}
