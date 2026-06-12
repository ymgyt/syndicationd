use std::{sync::Arc, time::Duration};

use synd_feed::feed::service::FeedService;
use tokio_util::sync::CancellationToken;

use crate::{
    command::{
        SubscribeFeedCommand, SubscribeFeedOutput, UnsubscribeFeedCommand, UnsubscribeFeedOutput,
    },
    config::FeedRegistryConfig,
    consumers::{ApiEventProj, SubRequestProj},
    crawl::{
        policy::CrawlPolicy, scheduler::CrawlScheduler, target_list::CrawlTargetListProj,
        worker::spawn_crawl_worker_pool,
    },
    db::{
        BlobStoreTx, CommitTx, CrawlCompletionTx, CrawlJobQueueTx, CrawlScheduleTx,
        EntryProjectionTx, FeedProjectionTx, FeedRegistryDb, RegistryTx, TimelineProjectionTx,
    },
    entry::EntryProj,
    error::FeedRegistryError,
    event::{
        ApiEventPublisher, ApiEventSubscriber, EventSubmitter, EventWakePublisher, Processor,
        WorkerHandle, WorkerPhase, WorkerSet, spawn_reconciler_worker, spawn_worker,
    },
    feed::FeedProj,
    query::{Subscriptions, SubscriptionsQuery, TimelineItemsPage, TimelineItemsQuery},
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
        + FeedProjectionTx
        + EntryProjectionTx
        + TimelineProjectionTx,
{
    pub fn start(db: S, config: FeedRegistryConfig, ct: CancellationToken) -> Self {
        let api_events = ApiEventPublisher::default();
        let wake_publisher = EventWakePublisher::new(config.event_wake_channel_capacity);
        let event_submitter = EventSubmitter::new(db.clone(), wake_publisher.clone());
        let registry =
            FeedRegistry::with_api_events(db.clone(), config, api_events.clone(), event_submitter);
        let workers = spawn_event_workers(db, &wake_publisher, api_events, config, ct);

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
) -> WorkerSet
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: BlobStoreTx
        + CrawlCompletionTx
        + CrawlScheduleTx
        + CrawlJobQueueTx
        + FeedProjectionTx
        + EntryProjectionTx
        + TimelineProjectionTx,
{
    let subscription_request_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.subscription_request_poll_interval,
        ct.clone(),
        SubRequestProj::new(),
    );
    let crawl_target_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.crawl_target_projection_poll_interval,
        ct.clone(),
        CrawlTargetListProj::new(),
    );
    let api_event_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.api_event_projection_poll_interval,
        ct.clone(),
        ApiEventProj::new(),
    );
    let feed_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.feed_projection_poll_interval,
        ct.clone(),
        FeedProj::new(),
    );
    let entry_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.entry_projection_poll_interval,
        ct.clone(),
        EntryProj::new(),
    );
    let timeline_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.timeline_projection_poll_interval,
        ct.clone(),
        TimelineProj::new(),
    );
    let api_event_publisher_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.api_event_publisher_poll_interval,
        ct.clone(),
        api_events,
    );
    let crawl_scheduler_worker = spawn_reconciler_worker(
        db.clone(),
        wake_publisher.clone(),
        config.workers.crawl_scheduler_poll_interval,
        ct.clone(),
        CrawlScheduler::new(),
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
) -> WorkerHandle
where
    S: FeedRegistryDb,
    P: Processor,
    P::Phase: WorkerPhase<S, P>,
{
    spawn_worker(db, wake_publisher, poll_interval, ct, processor)
}
