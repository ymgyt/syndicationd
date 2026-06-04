use chrono::Utc;

use crate::{
    consumers::unexpected_event,
    db::FeedRegistryDb,
    event::{
        ConsumeContext, Consumer, Event, EventInterests, FeedSubscribedEvent,
        FeedUnsubscribedEvent, Processor, ProcessorError, ProcessorId, ProcessorResult,
        RequestEvent, RequestEventKind, SubscribeFeedRequested, SubscriptionChangedEvent,
        Transactional, UnsubscribeFeedRejected, UnsubscribeFeedRequested,
    },
    subscription::{FeedSubscriptionAttrs, SubscribeOutcome, UnsubscribeOutcome},
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
            .subscribe_feed(subscription.feed_url, attrs, now)
            .await?;

        match outcome {
            SubscribeOutcome::Subscribed(subscription) => {
                cx.record_event(FeedSubscribedEvent::new(subscription).with_request_id(request_id))
                    .await
            }
            SubscribeOutcome::Changed(subscription) => {
                cx.record_event(
                    SubscriptionChangedEvent::new(subscription).with_request_id(request_id),
                )
                .await
            }
        }
    }

    async fn handle_unsubscribe<S>(
        &self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        event: UnsubscribeFeedRequested,
    ) -> ProcessorResult<()>
    where
        S: FeedRegistryDb,
    {
        let UnsubscribeFeedRequested {
            request_id,
            subscription,
        } = event;

        let outcome = cx
            .subscriber_scope(subscription.subscriber_id)
            .unsubscribe_feed(subscription.feed_url)
            .await?;

        match outcome {
            UnsubscribeOutcome::Unsubscribed(subscription) => {
                cx.record_event(
                    FeedUnsubscribedEvent::new(subscription).with_request_id(request_id),
                )
                .await
            }
            UnsubscribeOutcome::NotSubscribed(subscription) => {
                cx.record_event(UnsubscribeFeedRejected::new(
                    request_id,
                    subscription,
                    "not subscribed",
                ))
                .await
            }
        }
    }
}
