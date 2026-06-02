use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::{
    command::{
        SubscribeFeedCommand, SubscribeFeedOutput, UnsubscribeFeedCommand, UnsubscribeFeedOutput,
    },
    config::FeedRegistryConfig,
    consumers::{ApiEventProj, SubRequestProj},
    crawl::{policy::CrawlPolicy, target_list::CrawlTargetListProj},
    db::{CommitTx, FeedRegistryDb, RegistryTx},
    error::FeedRegistryError,
    event::{
        ApiEventPublisher, ApiEventSubscriber, EventSubmitter, EventWakePublisher, Processor,
        RequestEvent, RequestId, SubscribeFeedRequested, UnsubscribeFeedRequested, WorkerHandle,
        WorkerPhase, WorkerSet, spawn_worker,
    },
    query::{Subscriptions, SubscriptionsQuery},
    subscription::{SubscriberId, SubscriptionKey},
};

/// Owns a registry facade together with the workers required to process its events.
pub struct RegistryService<S> {
    registry: FeedRegistry<S>,
    workers: WorkerSet,
}

impl<S> RegistryService<S>
where
    S: FeedRegistryDb,
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
        let request_id = RequestId::generate();
        let subscription = SubscriptionKey::new(command.subscriber_id, command.feed_url);
        let event = RequestEvent::SubscribeFeedRequested(SubscribeFeedRequested::new(
            request_id.clone(),
            subscription.clone(),
            command.requirement,
            command.category,
            command.crawl_policy,
        ));
        self.events.submit(vec![event.into()]).await?;

        Ok(SubscribeFeedOutput {
            request_id,
            subscription,
        })
    }

    pub async fn unsubscribe(
        &self,
        command: UnsubscribeFeedCommand,
    ) -> Result<UnsubscribeFeedOutput, FeedRegistryError> {
        let request_id = RequestId::generate();
        let subscription = SubscriptionKey::new(command.subscriber_id, command.feed_url);
        let event = RequestEvent::UnsubscribeFeedRequested(UnsubscribeFeedRequested::new(
            request_id.clone(),
            subscription.clone(),
        ));
        self.events.submit(vec![event.into()]).await?;
        Ok(UnsubscribeFeedOutput {
            request_id,
            subscription,
        })
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
{
    let poll_interval = config.event_worker_poll_interval;

    let subscription_request_projection_worker = spawn_event_worker(
        db.clone(),
        wake_publisher.clone(),
        poll_interval,
        ct.clone(),
        SubRequestProj::new(),
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
        subscription_request_projection_worker,
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
    spawn_worker(db, wake_publisher, poll_interval, ct, processor)
}
