use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::{
    config::FeedRegistryConfig,
    consumers::{ApiEventProj, ApiEventStream, SubRequestWorker},
    crawl::target_list::CrawlTargetListProj,
    db::FeedRegistryDb,
    event::{
        ApiEventPublisher, EventConsumer, EventJournal, EventWakePublisher, Worker, WorkerHandle,
        WorkerSet,
    },
};

pub fn spawn_event_workers<S, J>(
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
    let poll_interval = config.event_worker_poll_interval;

    WorkerSet::new(vec![
        spawn_event_worker(
            &journal,
            &wake_publisher,
            poll_interval,
            &ct,
            SubRequestWorker::new(db.clone()),
        ),
        spawn_event_worker(
            &journal,
            &wake_publisher,
            poll_interval,
            &ct,
            CrawlTargetListProj::new(db.clone()),
        ),
        spawn_event_worker(
            &journal,
            &wake_publisher,
            poll_interval,
            &ct,
            ApiEventProj::new(db),
        ),
        spawn_event_worker(
            &journal,
            &wake_publisher,
            poll_interval,
            &ct,
            ApiEventStream::new(api_events),
        ),
    ])
}

fn spawn_event_worker<J, C>(
    journal: &J,
    wake_publisher: &EventWakePublisher,
    poll_interval: Duration,
    ct: &CancellationToken,
    consumer: C,
) -> WorkerHandle
where
    J: EventJournal,
    C: EventConsumer,
{
    Worker::new(
        journal.clone(),
        consumer,
        wake_publisher.clone(),
        wake_publisher.subscribe(),
        poll_interval,
    )
    .spawn(ct.clone())
}
