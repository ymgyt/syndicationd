use chrono::{DateTime, Utc};

use crate::{
    api::{
        ApiEvent, ApiFeedSubscribeRejected, ApiFeedSubscribed, ApiFeedSubscriptionChanged,
        ApiFeedUnsubscribeRejected, ApiFeedUnsubscribed, ApiTimelineChanged,
    },
    db::FeedRegistryDb,
    event::{
        ConsumeContext, Consumer, ConsumerInput, Event, EventType, FeedSubscribedEvent,
        FeedUnsubscribedEvent, Processor, ProcessorError, ProcessorId, ProcessorResult,
        RegistryEvent, SubscribeFeedRejected, SubscriptionChangedEvent, TimelineChangedEvent,
        UnsubscribeFeedRejected,
    },
};

/// Event input used to project public API events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiEventProjectionInput {
    SubscribeFeedRejected(SubscribeFeedRejected),
    UnsubscribeFeedRejected(UnsubscribeFeedRejected),
    FeedSubscribed(FeedSubscribedEvent),
    SubscriptionChanged(SubscriptionChangedEvent),
    FeedUnsubscribed(FeedUnsubscribedEvent),
    TimelineChanged(TimelineChangedEvent),
}

impl ApiEventProjectionInput {
    fn into_api_event(self) -> Option<ApiEvent> {
        match self {
            Self::SubscribeFeedRejected(event) => Some(ApiEvent::FeedSubscribeRejected(
                ApiFeedSubscribeRejected::new(event.request_id, event.subscription, event.reason),
            )),
            Self::UnsubscribeFeedRejected(event) => Some(ApiEvent::FeedUnsubscribeRejected(
                ApiFeedUnsubscribeRejected::new(event.request_id, event.subscription, event.reason),
            )),
            Self::FeedSubscribed(event) => {
                let request_id = event.request_id?;
                Some(ApiEvent::FeedSubscribed(ApiFeedSubscribed::new(
                    request_id,
                    event.subscription,
                )))
            }
            Self::SubscriptionChanged(event) => {
                let request_id = event.request_id?;
                Some(ApiEvent::FeedSubscriptionChanged(
                    ApiFeedSubscriptionChanged::new(request_id, event.subscription),
                ))
            }
            Self::FeedUnsubscribed(event) => {
                let request_id = event.request_id?;
                Some(ApiEvent::FeedUnsubscribed(ApiFeedUnsubscribed::new(
                    request_id,
                    event.subscription,
                )))
            }
            Self::TimelineChanged(event) => Some(ApiEvent::TimelineChanged(
                ApiTimelineChanged::new(event.timeline, event.changed_at, event.affected_feeds),
            )),
        }
    }
}

impl ConsumerInput for ApiEventProjectionInput {
    const INTERESTS: &'static [EventType] = &[
        SubscribeFeedRejected::TYPE,
        UnsubscribeFeedRejected::TYPE,
        FeedSubscribedEvent::TYPE,
        SubscriptionChangedEvent::TYPE,
        FeedUnsubscribedEvent::TYPE,
        TimelineChangedEvent::TYPE,
    ];

    fn from_event(event: Event, _occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::SubscribeFeedRejected(event) => Ok(Self::SubscribeFeedRejected(event)),
            Event::UnsubscribeFeedRejected(event) => Ok(Self::UnsubscribeFeedRejected(event)),
            Event::FeedSubscribed(event) => Ok(Self::FeedSubscribed(event)),
            Event::SubscriptionChanged(event) => Ok(Self::SubscriptionChanged(event)),
            Event::FeedUnsubscribed(event) => Ok(Self::FeedUnsubscribed(event)),
            Event::TimelineChanged(event) => Ok(Self::TimelineChanged(event)),
            event => Err(ProcessorError::unexpected_input(
                "api projection event",
                &event,
            )),
        }
    }
}

/// Projects request and subscription facts into public API events.
#[derive(Debug, Clone)]
pub struct ApiEventProj;

impl ApiEventProj {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApiEventProj {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for ApiEventProj {
    type Input = ApiEventProjectionInput;

    fn id(&self) -> ProcessorId {
        ProcessorId::ApiEventProjection
    }
}

impl<S> Consumer<S> for ApiEventProj
where
    S: FeedRegistryDb,
{
    async fn consume(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        let _ = cx;
        let Some(api_event) = input.into_api_event() else {
            return Ok(Vec::new());
        };
        Ok(vec![api_event.into()])
    }
}
