use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::{
    config::FeedRegistryConfig,
    consumers::{ApiEventProj, SubRequestWorker},
    crawl::target_list::CrawlTargetListProj,
    db::FeedRegistryDb,
    event::{
        ApiEventPublisher, EventWakePublisher, Processor, Worker, WorkerHandle, WorkerPhase,
        WorkerSet,
    },
};

pub fn spawn_event_workers<S>(
    db: S,
    wake_publisher: &EventWakePublisher,
    api_events: ApiEventPublisher,
    config: FeedRegistryConfig,
    ct: CancellationToken,
) -> WorkerSet
where
    S: FeedRegistryDb,
{
    let poll_interval = config.event_worker_poll_interval;

    let subscription_request_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        poll_interval,
        ct.clone(),
        SubRequestWorker::new(),
    );
    let crawl_target_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        poll_interval,
        ct.clone(),
        CrawlTargetListProj::new(),
    );
    let api_event_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        poll_interval,
        ct.clone(),
        ApiEventProj::new(),
    );
    let api_event_publisher_worker =
        spawn_event_worker(db, wake_publisher.clone(), poll_interval, ct, api_events);

    WorkerSet::new(vec![
        subscription_request_worker,
        crawl_target_projection_worker,
        api_event_projection_worker,
        api_event_publisher_worker,
    ])
}

fn spawn_event_worker<S, P>(
    db: S,
    wake_publisher: EventWakePublisher,
    poll_interval: Duration,
    ct: CancellationToken,
    processor: P,
) -> WorkerHandle
where
    S: FeedRegistryDb,
    P: Processor,
    P::Phase: WorkerPhase<S, P>,
{
    let wake_subscriber = wake_publisher.subscribe();

    Worker::new(
        db,
        processor,
        wake_publisher,
        wake_subscriber,
        poll_interval,
    )
    .spawn(ct)
}
