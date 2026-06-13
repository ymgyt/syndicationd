use chrono::{DateTime, Utc};

use crate::{
    db::{FeedRegistryDb, SubscriptionTx},
    event::{
        ConsumeContext, Consumer, ConsumerInput, Event, EventType, FeedSubscribedEvent,
        FeedUnsubscribedEvent, Processor, ProcessorError, ProcessorId, ProcessorResult,
        RegistryEvent, SubscribeFeedRequested, SubscriptionChangedEvent, UnsubscribeFeedRejected,
        UnsubscribeFeedRequested,
    },
    subscription::{FeedSubscriptionAttrs, SubscribeOutcome, UnsubscribeOutcome},
};

/// Subscription request lifecycle events accepted by the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubRequestInput {
    /// A request to subscribe one subscriber to one feed.
    Subscribe {
        event: SubscribeFeedRequested,
        occurred_at: DateTime<Utc>,
    },
    /// A request to unsubscribe one subscriber from one feed.
    Unsubscribe {
        event: UnsubscribeFeedRequested,
        occurred_at: DateTime<Utc>,
    },
}

impl ConsumerInput for SubRequestInput {
    const INTERESTS: &'static [EventType] =
        &[SubscribeFeedRequested::TYPE, UnsubscribeFeedRequested::TYPE];

    fn from_event(event: Event, occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::SubscribeFeedRequested(event) => Ok(Self::Subscribe { event, occurred_at }),
            Event::UnsubscribeFeedRequested(event) => Ok(Self::Unsubscribe { event, occurred_at }),
            event => Err(ProcessorError::unexpected_input(
                "subscription request event",
                &event,
            )),
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

    fn id(&self) -> ProcessorId {
        ProcessorId::SubscriptionRequest
    }
}

impl<S> Consumer<S> for SubRequestProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: SubscriptionTx + Send,
{
    async fn consume(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        match input {
            SubRequestInput::Subscribe { event, occurred_at } => {
                self.handle_subscribe::<S>(cx, event, occurred_at).await
            }
            SubRequestInput::Unsubscribe { event, occurred_at } => {
                self.handle_unsubscribe::<S>(cx, event, occurred_at).await
            }
        }
    }
}

impl SubRequestProj {
    async fn handle_subscribe<S>(
        &self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        event: SubscribeFeedRequested,
        occurred_at: DateTime<Utc>,
    ) -> ProcessorResult<Vec<Event>>
    where
        S: FeedRegistryDb,
        for<'tx> S::Tx<'tx>: SubscriptionTx + Send,
    {
        let SubscribeFeedRequested {
            request_id,
            subscription,
            requirement,
            category,
            crawl_policy,
        } = event;
        let attrs = FeedSubscriptionAttrs {
            requirement,
            category,
            crawl_policy,
        };

        let outcome = cx
            .subscriber_scope(subscription.subscriber_id)
            .subscribe_feed(subscription.feed_url, attrs, occurred_at)
            .await?;

        let event = match outcome {
            SubscribeOutcome::Subscribed(subscription) => FeedSubscribedEvent::new(subscription)
                .with_request_id(request_id)
                .into(),
            SubscribeOutcome::Changed(subscription) => SubscriptionChangedEvent::new(subscription)
                .with_request_id(request_id)
                .into(),
        };
        Ok(vec![event])
    }

    async fn handle_unsubscribe<S>(
        &self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        event: UnsubscribeFeedRequested,
        _occurred_at: DateTime<Utc>,
    ) -> ProcessorResult<Vec<Event>>
    where
        S: FeedRegistryDb,
        for<'tx> S::Tx<'tx>: SubscriptionTx + Send,
    {
        let UnsubscribeFeedRequested {
            request_id,
            subscription,
        } = event;

        let outcome = cx
            .subscriber_scope(subscription.subscriber_id)
            .unsubscribe_feed(subscription.feed_url)
            .await?;

        let event = match outcome {
            UnsubscribeOutcome::Unsubscribed(subscription) => {
                FeedUnsubscribedEvent::new(subscription)
                    .with_request_id(request_id)
                    .into()
            }
            UnsubscribeOutcome::NotSubscribed(subscription) => {
                UnsubscribeFeedRejected::new(request_id, subscription, "not subscribed").into()
            }
        };
        Ok(vec![event])
    }
}
