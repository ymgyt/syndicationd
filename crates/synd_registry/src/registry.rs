use crate::{
    command::{
        SubscribeFeedCommand, SubscribeFeedOutput, UnsubscribeFeedCommand, UnsubscribeFeedOutput,
    },
    config::FeedRegistryConfig,
    crawl::policy::RefreshPolicy,
    db::{CommitTx, FeedRegistryDb, RegistryTx},
    error::FeedRegistryError,
    event::{
        ApiEventPublisher, ApiEventSubscriber, EventSubmitter, RequestEvent, RequestId,
        SubscribeFeedRequested, UnsubscribeFeedRequested,
    },
    query::{Subscriptions, SubscriptionsQuery},
    subscription::{SubscriberId, SubscriptionKey},
};

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

    pub fn default_refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::interval(self.config.default_refresh_interval)
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
            command.refresh_policy,
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
