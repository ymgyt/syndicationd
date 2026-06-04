use crate::{
    consumers::unexpected_event,
    db::FeedRegistryDb,
    event::{
        ApiEvent, ApiFeedSubscribeRejected, ApiFeedSubscribed, ApiFeedSubscriptionChanged,
        ApiFeedUnsubscribeRejected, ApiFeedUnsubscribed, ConsumeContext, Consumer, Event,
        EventInterests, Processor, ProcessorError, ProcessorId, ProcessorResult, RequestEvent,
        RequestEventKind, SubEvent, SubEventKind, Transactional,
    },
};

/// Event input used to project public API events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiEventProjectionInput {
    event: Event,
}

impl ApiEventProjectionInput {
    pub fn new(event: Event) -> Self {
        Self { event }
    }

    pub fn into_event(self) -> Event {
        self.event
    }
}

impl TryFrom<Event> for ApiEventProjectionInput {
    type Error = ProcessorError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            event @ (Event::Request(
                RequestEvent::SubscribeFeedRejected(_) | RequestEvent::UnsubscribeFeedRejected(_),
            )
            | Event::Sub(
                SubEvent::FeedSubscribed(_)
                | SubEvent::SubscriptionChanged(_)
                | SubEvent::FeedUnsubscribed(_),
            )) => Ok(Self::new(event)),
            event => Err(unexpected_event("api projection event", &event)),
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
    type Phase = Transactional;

    fn id(&self) -> ProcessorId {
        ProcessorId::ApiEventProjection
    }

    fn interests(&self) -> EventInterests {
        EventInterests::new([
            RequestEventKind::SubscribeFeedRejected.into(),
            RequestEventKind::UnsubscribeFeedRejected.into(),
            SubEventKind::FeedSubscribed.into(),
            SubEventKind::SubscriptionChanged.into(),
            SubEventKind::FeedUnsubscribed.into(),
        ])
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
    ) -> ProcessorResult<()> {
        let Some(api_event) = project_api_event(input.into_event()) else {
            return Ok(());
        };
        cx.record_event(api_event).await
    }
}

fn project_api_event(event: Event) -> Option<ApiEvent> {
    match event {
        Event::Request(RequestEvent::SubscribeFeedRejected(event)) => {
            Some(ApiEvent::FeedSubscribeRejected(
                ApiFeedSubscribeRejected::new(event.request_id, event.subscription, event.reason),
            ))
        }
        Event::Request(RequestEvent::UnsubscribeFeedRejected(event)) => {
            Some(ApiEvent::FeedUnsubscribeRejected(
                ApiFeedUnsubscribeRejected::new(event.request_id, event.subscription, event.reason),
            ))
        }
        Event::Sub(SubEvent::FeedSubscribed(event)) => {
            let request_id = event.request_id?;
            Some(ApiEvent::FeedSubscribed(ApiFeedSubscribed::new(
                request_id,
                event.subscription,
            )))
        }
        Event::Sub(SubEvent::SubscriptionChanged(event)) => {
            let request_id = event.request_id?;
            Some(ApiEvent::FeedSubscriptionChanged(
                ApiFeedSubscriptionChanged::new(request_id, event.subscription),
            ))
        }
        Event::Sub(SubEvent::FeedUnsubscribed(event)) => {
            let request_id = event.request_id?;
            Some(ApiEvent::FeedUnsubscribed(ApiFeedUnsubscribed::new(
                request_id,
                event.subscription,
            )))
        }
        _ => None,
    }
}
