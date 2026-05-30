use chrono::Utc;

use crate::{
    command::{
        SubscribeFeedCommand, SubscribeFeedOutput, UnsubscribeFeedCommand, UnsubscribeFeedOutput,
    },
    config::FeedRegistryConfig,
    crawl::policy::RefreshPolicy,
    db::{FeedRegistryDb, RegistryDbTransaction},
    error::FeedRegistryError,
    event::{
        ApiEventPublisher, ApiEventSubscriber, EventSubmitter, RequestEvent, RequestId,
        SubscribeFeedRequested, UnsubscribeFeedRequested,
    },
    subscriber::SubscriberId,
    subscription::{Subscription, SubscriptionKey},
    view::{Subscriptions, SubscriptionsQuery},
};

#[derive(Clone)]
pub struct FeedRegistry<S, E> {
    db: S,
    config: FeedRegistryConfig,
    api_events: ApiEventPublisher,
    events: E,
}

impl<S, E> FeedRegistry<S, E>
where
    S: FeedRegistryDb,
    E: EventSubmitter,
{
    pub fn new(db: S, config: FeedRegistryConfig, events: E) -> Self {
        Self::with_event_runtime(db, config, ApiEventPublisher::default(), events)
    }

    pub fn with_event_runtime(
        db: S,
        config: FeedRegistryConfig,
        api_events: ApiEventPublisher,
        events: E,
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
        let subscription =
            SubscriptionKey::new(command.subscriber_id.clone(), command.feed_url.clone());
        let event = RequestEvent::SubscribeFeedRequested(SubscribeFeedRequested::new(
            request_id.clone(),
            subscription,
            command.requirement,
            command.category.clone(),
            command.refresh_policy,
        ));
        self.events.submit(vec![event.into()]).await?;

        let now = Utc::now();
        Ok(SubscribeFeedOutput {
            subscription: Subscription {
                subscriber_id: command.subscriber_id,
                feed_url: command.feed_url,
                requirement: command.requirement,
                category: command.category,
                refresh_policy: command.refresh_policy,
                created_at: now,
                updated_at: now,
            },
            request_id,
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
            subscription,
        ));
        self.events.submit(vec![event.into()]).await?;
        Ok(UnsubscribeFeedOutput {
            request_id: Some(request_id),
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
