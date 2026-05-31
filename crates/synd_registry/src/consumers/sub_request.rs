use chrono::Utc;

use crate::{
    consumers::unexpected_event,
    db::{FeedRegistryDb, RegistryTx},
    event::{
        ConsumeContext, Consumer, Event, EventInterests, FeedSubscribed, FeedUnsubscribed,
        Processor, ProcessorError, ProcessorId, ProcessorResult, RequestEvent, RequestEventKind,
        SubEvent, SubscribeFeedRequested, SubscriptionChanged, Transactional,
        UnsubscribeFeedRejected, UnsubscribeFeedRequested,
    },
    subscription::Subscription,
};

/// Subscription request lifecycle events accepted by the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubRequestInput {
    /// A request to subscribe one subscriber to one feed.
    Subscribe(SubscribeFeedRequested),
    /// A request to unsubscribe one subscriber from one feed.
    Unsubscribe(UnsubscribeFeedRequested),
}

impl TryFrom<Event> for SubRequestInput {
    type Error = ProcessorError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Request(RequestEvent::SubscribeFeedRequested(event)) => {
                Ok(Self::Subscribe(event))
            }
            Event::Request(RequestEvent::UnsubscribeFeedRequested(event)) => {
                Ok(Self::Unsubscribe(event))
            }
            event => Err(unexpected_event("subscription request event", &event)),
        }
    }
}

/// Turns subscription request events into subscription domain events.
#[derive(Debug, Clone)]
pub struct SubRequestProj;

impl SubRequestProj {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SubRequestProj {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for SubRequestProj {
    type Input = SubRequestInput;
    type Phase = Transactional;

    fn id(&self) -> ProcessorId {
        ProcessorId::SubscriptionRequest
    }

    fn interests(&self) -> EventInterests {
        EventInterests::new([
            RequestEventKind::SubscribeFeedRequested.into(),
            RequestEventKind::UnsubscribeFeedRequested.into(),
        ])
    }
}

impl<S> Consumer<S> for SubRequestProj
where
    S: FeedRegistryDb,
{
    async fn consume(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        input: Self::Input,
    ) -> ProcessorResult<()> {
        match input {
            SubRequestInput::Subscribe(event) => self.handle_subscribe::<S>(cx, event).await?,
            SubRequestInput::Unsubscribe(event) => self.handle_unsubscribe::<S>(cx, event).await?,
        }
        Ok(())
    }
}

impl SubRequestProj {
    async fn handle_subscribe<S>(
        &self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        event: SubscribeFeedRequested,
    ) -> ProcessorResult<()>
    where
        S: FeedRegistryDb,
    {
        let now = Utc::now();
        let subscription = Subscription {
            subscriber_id: event.subscription.subscriber_id.clone(),
            feed_url: event.subscription.feed_url.clone(),
            requirement: event.requirement,
            category: event.category,
            refresh_policy: event.refresh_policy,
            created_at: now,
            updated_at: now,
        };

        let already_subscribed = cx
            .has_subscription(&subscription.subscriber_id, &subscription.feed_url)
            .await?;
        cx.upsert_subscription(subscription).await?;

        let event = if already_subscribed {
            Event::Sub(SubEvent::SubscriptionChanged(
                SubscriptionChanged::new(event.subscription).with_request_id(event.request_id),
            ))
        } else {
            Event::Sub(SubEvent::FeedSubscribed(
                FeedSubscribed::new(event.subscription).with_request_id(event.request_id),
            ))
        };
        cx.record_event(event).await
    }

    async fn handle_unsubscribe<S>(
        &self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        event: UnsubscribeFeedRequested,
    ) -> ProcessorResult<()>
    where
        S: FeedRegistryDb,
    {
        let is_subscribed = cx
            .has_subscription(
                &event.subscription.subscriber_id,
                &event.subscription.feed_url,
            )
            .await?;

        let event = if is_subscribed {
            cx.delete_subscription(
                &event.subscription.subscriber_id,
                &event.subscription.feed_url,
            )
            .await?;
            Event::Sub(SubEvent::FeedUnsubscribed(
                FeedUnsubscribed::new(event.subscription).with_request_id(event.request_id),
            ))
        } else {
            Event::Request(RequestEvent::UnsubscribeFeedRejected(
                UnsubscribeFeedRejected::new(
                    event.request_id,
                    event.subscription,
                    "not subscribed",
                ),
            ))
        };

        cx.record_event(event).await
    }
}
