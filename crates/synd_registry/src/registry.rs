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
        policy::CrawlPolicy, scheduler::CrawlScheduler, target_list::CrawlTargetListProj,
        worker::spawn_crawl_worker_pool,
    },
    db::{
        BlobStoreTx, CommitTx, CrawlCompletionTx, CrawlJobQueueTx, CrawlScheduleTx, CrawlTargetTx,
        EntryProjectionTx, FeedProjectionTx, FeedRegistryDb, SubscriptionTx, TimelineTx,
    },
    entry::EntryProj,
    error::FeedRegistryError,
    event::{
        Consumer, CursorAdapter, EventSubmitter, EventWakePublisher, JournalAppendTx,
        PostCommitAdapter, Processor, Sink, WorkerHandle, WorkerSet, spawn_event_loop,
    },
    feed::FeedProj,
    query::{Subscriptions, SubscriptionsQuery, TimelineItemsPage, TimelineItemsQuery},
    subscription::SubRequestProj,
    subscription::SubscriberId,
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
    for<'tx> S::Tx<'tx>: BlobStoreTx
        + CrawlCompletionTx
        + CrawlScheduleTx
        + CrawlJobQueueTx
        + CrawlTargetTx
        + FeedProjectionTx
        + EntryProjectionTx
        + SubscriptionTx
        + TimelineTx
        + JournalAppendTx,
{
    pub fn start(db: S, config: FeedRegistryConfig, ct: CancellationToken) -> Self {
        let api_events = ApiEventPublisher::default();
        let wake_publisher = EventWakePublisher::new(config.event_wake_channel_capacity);
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let event_submitter =
            EventSubmitter::with_clock(db.clone(), wake_publisher.clone(), Arc::clone(&clock));
        let registry =
            FeedRegistry::with_api_events(db.clone(), config, api_events.clone(), event_submitter);
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
    events: EventSubmitter<S>,
}

impl<S> FeedRegistry<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: JournalAppendTx,
{
    pub fn new(db: S, config: FeedRegistryConfig, events: EventSubmitter<S>) -> Self {
        Self::with_api_events(db, config, ApiEventPublisher::default(), events)
    }

    pub fn with_api_events(
        db: S,
        config: FeedRegistryConfig,
        api_events: ApiEventPublisher,
        events: EventSubmitter<S>,
    ) -> Self {
        Self {
            db,
            config,
            api_events,
            events,
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
        let request = command.into_request();
        self.events.submit([request.clone()]).await?;
        Ok(request.into())
    }

    pub async fn unsubscribe(
        &self,
        command: UnsubscribeFeedCommand,
    ) -> Result<UnsubscribeFeedOutput, FeedRegistryError> {
        let request = command.into_request();
        self.events.submit([request.clone()]).await?;
        Ok(request.into())
    }
}

impl<S> FeedRegistry<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: SubscriptionTx + TimelineTx,
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
    for<'tx> S::Tx<'tx>: BlobStoreTx
        + CrawlCompletionTx
        + CrawlScheduleTx
        + CrawlJobQueueTx
        + CrawlTargetTx
        + FeedProjectionTx
        + EntryProjectionTx
        + SubscriptionTx
        + TimelineTx
        + JournalAppendTx,
{
    let subscription_request_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.subscription_request_poll_interval,
        ct.clone(),
        SubRequestProj::new(),
        Arc::clone(&clock),
    );
    let crawl_target_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.crawl_target_projection_poll_interval,
        ct.clone(),
        CrawlTargetListProj::new(),
        Arc::clone(&clock),
    );
    let api_event_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.api_event_projection_poll_interval,
        ct.clone(),
        ApiEventProj::new(),
        Arc::clone(&clock),
    );
    let feed_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.feed_projection_poll_interval,
        ct.clone(),
        FeedProj::new(),
        Arc::clone(&clock),
    );
    let entry_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.entry_projection_poll_interval,
        ct.clone(),
        EntryProj::new(),
        Arc::clone(&clock),
    );
    let timeline_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.timeline_projection_poll_interval,
        ct.clone(),
        TimelineProj::new(),
        Arc::clone(&clock),
    );
    let api_event_publisher_worker = spawn_post_commit_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.api_event_publisher_poll_interval,
        ct.clone(),
        api_events,
    );
    let crawl_scheduler_worker = spawn_event_loop(
        CrawlScheduler::with_clock(db.clone(), Arc::clone(&clock)),
        wake_publisher.clone(),
        config.workers.crawl_scheduler_poll_interval,
        ct.clone(),
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
        subscription_request_projection_worker,
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
    P: Consumer<S>,
    for<'tx> S::Tx<'tx>: JournalAppendTx,
{
    spawn_event_loop(
        CursorAdapter::new(db, processor, clock),
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
