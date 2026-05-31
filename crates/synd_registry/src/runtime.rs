use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::{
    config::FeedRegistryConfig,
    consumers::{ApiEventProj, SubRequestWorker},
    crawl::target_list::CrawlTargetListProj,
    db::FeedRegistryDb,
    event::{
        ApiEventPublisher, Consumer, EventJournal, EventWakePublisher, Sink, SinkWorker, Worker,
        WorkerHandle, WorkerSet,
    },
};

pub fn spawn_event_workers<S, J>(
    db: S,
    journal: J,
    wake_publisher: &EventWakePublisher,
    api_events: ApiEventPublisher,
    config: FeedRegistryConfig,
    ct: CancellationToken,
) -> WorkerSet
where
    S: FeedRegistryDb,
    J: EventJournal,
{
    let poll_interval = config.event_worker_poll_interval;
    let subscription_request_worker = {
        let consumer = SubRequestWorker::new();
        spawn_event_worker(
            db.clone(),
            journal.clone(),
            wake_publisher.clone(),
            poll_interval,
            ct.clone(),
            consumer,
        )
    };
    let crawl_target_projection_worker = {
        let consumer = CrawlTargetListProj::new();
        spawn_event_worker(
            db.clone(),
            journal.clone(),
            wake_publisher.clone(),
            poll_interval,
            ct.clone(),
            consumer,
        )
    };
    let api_event_projection_worker = {
        let consumer = ApiEventProj::new();
        spawn_event_worker(
            db.clone(),
            journal.clone(),
            wake_publisher.clone(),
            poll_interval,
            ct.clone(),
            consumer,
        )
    };
    let api_event_publisher_worker = {
        let sink = api_events;
        spawn_event_sink_worker(db, journal, wake_publisher, poll_interval, ct, sink)
    };

    WorkerSet::new(vec![
        subscription_request_worker,
        crawl_target_projection_worker,
        api_event_projection_worker,
        api_event_publisher_worker,
    ])
}

fn spawn_event_worker<S, J, C>(
    db: S,
    journal: J,
    wake_publisher: EventWakePublisher,
    poll_interval: Duration,
    ct: CancellationToken,
    consumer: C,
) -> WorkerHandle
where
    S: FeedRegistryDb,
    J: EventJournal,
    C: Consumer<S>,
{
    let wake_subscriber = wake_publisher.subscribe();

    Worker::new(
        db,
        journal,
        consumer,
        wake_publisher,
        wake_subscriber,
        poll_interval,
    )
    .spawn(ct)
}

fn spawn_event_sink_worker<S, J, K>(
    db: S,
    journal: J,
    wake_publisher: &EventWakePublisher,
    poll_interval: Duration,
    ct: CancellationToken,
    sink: K,
) -> WorkerHandle
where
    S: FeedRegistryDb,
    J: EventJournal,
    K: Sink,
{
    let wake_subscriber = wake_publisher.subscribe();

    SinkWorker::new(db, journal, sink, wake_subscriber, poll_interval).spawn(ct)
}
