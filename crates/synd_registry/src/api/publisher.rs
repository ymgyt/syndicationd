use std::fmt;

use chrono::{DateTime, Utc};
use tokio::sync::broadcast;
use tracing::debug;

use crate::{
    SubscriberId,
    api::{
        ApiCrawlJobEnqueued, ApiCrawlJobFinished, ApiCrawlJobStarted, ApiEntryChanged,
        ApiEntryDiscovered, ApiEvent, ApiFeedChanged, ApiFeedDiscovered, ApiFeedSubscribeRejected,
        ApiFeedSubscribed, ApiFeedSubscriptionChanged, ApiFeedUnsubscribeRejected,
        ApiFeedUnsubscribed, ApiTimelineChanged,
    },
    event::{
        ConsumerInput, Event, EventType, Processor, ProcessorError, ProcessorId, ProcessorResult,
        RegistryEvent, Sink,
    },
};

/// Broadcasts committed API events to subscriber-scoped listeners.
#[derive(Clone)]
pub struct ApiEventPublisher {
    sender: broadcast::Sender<ApiEvent>,
}

/// Receives API events for one subscriber.
pub struct ApiEventSubscriber {
    subscriber_id: SubscriberId,
    receiver: broadcast::Receiver<ApiEvent>,
}

/// Error returned while receiving API events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiEventRecvError {
    Closed,
    Lagged(u64),
}

impl ApiEventPublisher {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self, subscriber_id: SubscriberId) -> ApiEventSubscriber {
        ApiEventSubscriber {
            subscriber_id,
            receiver: self.sender.subscribe(),
        }
    }

    pub fn publish(&self, event: ApiEvent) -> usize {
        self.sender.send(event).unwrap_or_default()
    }
}

impl Default for ApiEventPublisher {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl Processor for ApiEventPublisher {
    type Input = ApiEvent;

    fn id(&self) -> ProcessorId {
        ProcessorId::ApiEventPublisher
    }
}

impl Sink for ApiEventPublisher {
    async fn consume(&mut self, input: Self::Input) -> ProcessorResult<()> {
        let receivers = self.publish(input);
        debug!(receivers, "registry api event publisher delivered event");
        Ok(())
    }
}

impl ConsumerInput for ApiEvent {
    const INTERESTS: &'static [EventType] = &[
        ApiFeedSubscribed::TYPE,
        ApiFeedSubscribeRejected::TYPE,
        ApiFeedSubscriptionChanged::TYPE,
        ApiFeedUnsubscribed::TYPE,
        ApiFeedUnsubscribeRejected::TYPE,
        ApiCrawlJobEnqueued::TYPE,
        ApiCrawlJobStarted::TYPE,
        ApiCrawlJobFinished::TYPE,
        ApiFeedDiscovered::TYPE,
        ApiFeedChanged::TYPE,
        ApiEntryDiscovered::TYPE,
        ApiEntryChanged::TYPE,
        ApiTimelineChanged::TYPE,
    ];

    fn from_event(event: Event, _occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::ApiFeedSubscribed(event) => Ok(Self::FeedSubscribed(event)),
            Event::ApiFeedSubscribeRejected(event) => Ok(Self::FeedSubscribeRejected(event)),
            Event::ApiFeedSubscriptionChanged(event) => Ok(Self::FeedSubscriptionChanged(event)),
            Event::ApiFeedUnsubscribed(event) => Ok(Self::FeedUnsubscribed(event)),
            Event::ApiFeedUnsubscribeRejected(event) => Ok(Self::FeedUnsubscribeRejected(event)),
            Event::ApiCrawlJobEnqueued(event) => Ok(Self::CrawlJobEnqueued(event)),
            Event::ApiCrawlJobStarted(event) => Ok(Self::CrawlJobStarted(event)),
            Event::ApiCrawlJobFinished(event) => Ok(Self::CrawlJobFinished(event)),
            Event::ApiFeedDiscovered(event) => Ok(Self::FeedDiscovered(event)),
            Event::ApiFeedChanged(event) => Ok(Self::FeedChanged(event)),
            Event::ApiEntryDiscovered(event) => Ok(Self::EntryDiscovered(event)),
            Event::ApiEntryChanged(event) => Ok(Self::EntryChanged(event)),
            Event::ApiTimelineChanged(event) => Ok(Self::TimelineChanged(event)),
            event => Err(ProcessorError::unexpected_input("api event", &event)),
        }
    }
}

impl fmt::Debug for ApiEventPublisher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiEventPublisher").finish_non_exhaustive()
    }
}

impl ApiEventSubscriber {
    pub async fn recv(&mut self) -> Result<ApiEvent, ApiEventRecvError> {
        loop {
            let event = self.receiver.recv().await.map_err(|err| match err {
                broadcast::error::RecvError::Closed => ApiEventRecvError::Closed,
                broadcast::error::RecvError::Lagged(skipped) => ApiEventRecvError::Lagged(skipped),
            })?;
            if event_subscriber_id(&event) == &self.subscriber_id {
                return Ok(event);
            }
        }
    }
}

fn event_subscriber_id(event: &ApiEvent) -> &SubscriberId {
    match event {
        ApiEvent::FeedSubscribed(event) => &event.subscription.subscriber_id,
        ApiEvent::FeedSubscribeRejected(event) => &event.subscription.subscriber_id,
        ApiEvent::FeedSubscriptionChanged(event) => &event.subscription.subscriber_id,
        ApiEvent::FeedUnsubscribed(event) => &event.subscription.subscriber_id,
        ApiEvent::FeedUnsubscribeRejected(event) => &event.subscription.subscriber_id,
        ApiEvent::CrawlJobEnqueued(event) => &event.subscriber_id,
        ApiEvent::CrawlJobStarted(event) => &event.subscriber_id,
        ApiEvent::CrawlJobFinished(event) => &event.subscriber_id,
        ApiEvent::FeedDiscovered(event) => &event.subscriber_id,
        ApiEvent::FeedChanged(event) => &event.subscriber_id,
        ApiEvent::EntryDiscovered(event) => &event.subscriber_id,
        ApiEvent::EntryChanged(event) => &event.subscriber_id,
        ApiEvent::TimelineChanged(event) => &event.timeline.subscriber_id,
    }
}
