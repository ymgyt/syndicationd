use chrono::Utc;

use crate::{
    consumers::unexpected_event,
    db::{FeedRegistryDb, RegistryTx},
    event::{
        Consumer, Event, EventInterests, EventKind, FeedSubscribed, FeedUnsubscribed, JournalTx,
        Processor, ProcessorError, ProcessorId, ProcessorResult, RecordedEvents, RequestEvent,
        RequestEventKind, SubEvent, SubscribeFeedRequested, SubscriptionChanged, Transactional,
        UnsubscribeFeedRejected, UnsubscribeFeedRequested,
    },
    subscription::Subscription,
};

/// Subscription request lifecycle events accepted by the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubRequestInput {
    event: RequestEvent,
}

impl SubRequestInput {
    pub fn new(event: RequestEvent) -> Self {
        Self { event }
    }

    pub fn into_event(self) -> RequestEvent {
        self.event
    }
}

impl TryFrom<Event> for SubRequestInput {
    type Error = ProcessorError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Request(
                event @ (RequestEvent::SubscribeFeedRequested(_)
                | RequestEvent::UnsubscribeFeedRequested(_)),
            ) => Ok(Self::new(event)),
            event => Err(unexpected_event("subscription request event", &event)),
        }
    }
}

/// Turns subscription request events into subscription domain events.
#[derive(Debug, Clone)]
pub struct SubRequestWorker;

impl SubRequestWorker {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SubRequestWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for SubRequestWorker {
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

impl<S> Consumer<S> for SubRequestWorker
where
    S: FeedRegistryDb,
{
    async fn consume(
        &mut self,
        tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> ProcessorResult<RecordedEvents> {
        let mut recorded = RecordedEvents::empty();
        let kind = match input.into_event() {
            RequestEvent::SubscribeFeedRequested(event) => self.handle_subscribe(tx, event).await?,
            RequestEvent::UnsubscribeFeedRequested(event) => {
                self.handle_unsubscribe(tx, event).await?
            }
            event => unreachable!("unexpected subscription request event: {event:?}"),
        };
        recorded.push(kind);
        Ok(recorded)
    }
}

impl SubRequestWorker {
    async fn handle_subscribe<T>(
        &self,
        tx: &mut T,
        event: SubscribeFeedRequested,
    ) -> ProcessorResult<EventKind>
    where
        T: RegistryTx + JournalTx + Send,
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

        let already_subscribed = tx
            .has_subscription(&subscription.subscriber_id, &subscription.feed_url)
            .await?;
        tx.upsert_subscription(subscription).await?;

        let event = if already_subscribed {
            Event::Sub(SubEvent::SubscriptionChanged(
                SubscriptionChanged::new(event.subscription).with_request_id(event.request_id),
            ))
        } else {
            Event::Sub(SubEvent::FeedSubscribed(
                FeedSubscribed::new(event.subscription).with_request_id(event.request_id),
            ))
        };
        let kind = event.kind();
        tx.append_event(event).await?;
        Ok(kind)
    }

    async fn handle_unsubscribe<T>(
        &self,
        tx: &mut T,
        event: UnsubscribeFeedRequested,
    ) -> ProcessorResult<EventKind>
    where
        T: RegistryTx + JournalTx + Send,
    {
        let is_subscribed = tx
            .has_subscription(
                &event.subscription.subscriber_id,
                &event.subscription.feed_url,
            )
            .await?;

        let event = if is_subscribed {
            tx.delete_subscription(
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

        let kind = event.kind();
        tx.append_event(event).await?;
        Ok(kind)
    }
}
