use chrono::Utc;
use tracing::debug;

use crate::{
    db::{FeedRegistryDb, RegistryDbTransaction},
    event::{
        ApiEvent, ApiEventKind, ApiEventPublisher, ApiFeedSubscribeRejected, ApiFeedSubscribed,
        ApiFeedSubscriptionChanged, ApiFeedUnsubscribeRejected, ApiFeedUnsubscribed,
        ConsumerEventInput, Event, EventConsumer, EventConsumerId, EventConsumerResult, EventKind,
        EventReadBatch, EventReadFilter, FeedSubscribed, FeedUnsubscribed, JournaledEvent,
        RecordedEvents, RequestEvent, RequestEventKind, SubEvent, SubEventKind,
        SubscribeFeedRequested, SubscriptionChanged, UnsubscribeFeedRejected,
        UnsubscribeFeedRequested,
    },
    subscription::Subscription,
};

/// Subscription request lifecycle events accepted by the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubRequestInput {
    events: Vec<RequestEvent>,
}

impl SubRequestInput {
    pub fn new(events: Vec<RequestEvent>) -> Self {
        Self { events }
    }

    pub fn into_events(self) -> Vec<RequestEvent> {
        self.events
    }
}

impl ConsumerEventInput for SubRequestInput {
    const READ_FILTER: EventReadFilter = EventReadFilter::new(&[
        EventKind::Request(RequestEventKind::SubscribeFeedRequested),
        EventKind::Request(RequestEventKind::UnsubscribeFeedRequested),
    ]);

    fn from_batch(batch: EventReadBatch) -> EventConsumerResult<Option<Self>> {
        let events = batch
            .into_events()
            .into_iter()
            .map(JournaledEvent::into_event)
            .map(|event| match event {
                Event::Request(event) => event,
                event => unreachable!("unexpected subscription request event: {event:?}"),
            })
            .collect::<Vec<_>>();

        Ok((!events.is_empty()).then_some(Self::new(events)))
    }
}

/// Worker that turns subscription request events into subscription domain events.
#[derive(Debug, Clone)]
pub struct SubRequestWorker<S> {
    db: S,
}

impl<S> SubRequestWorker<S> {
    pub fn new(db: S) -> Self {
        Self { db }
    }
}

impl<S> EventConsumer for SubRequestWorker<S>
where
    S: FeedRegistryDb,
{
    type Input = SubRequestInput;

    fn id(&self) -> EventConsumerId {
        EventConsumerId::SubRequestWorker
    }

    async fn consume(&mut self, input: Self::Input) -> EventConsumerResult<RecordedEvents> {
        let mut recorded = RecordedEvents::empty();
        for event in input.into_events() {
            let kind = match event {
                RequestEvent::SubscribeFeedRequested(event) => self.handle_subscribe(event).await?,
                RequestEvent::UnsubscribeFeedRequested(event) => {
                    self.handle_unsubscribe(event).await?
                }
                event => unreachable!("unexpected subscription request event: {event:?}"),
            };
            recorded.push(kind);
        }
        Ok(recorded)
    }
}

impl<S> SubRequestWorker<S>
where
    S: FeedRegistryDb,
{
    async fn handle_subscribe(
        &self,
        event: SubscribeFeedRequested,
    ) -> EventConsumerResult<EventKind> {
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

        let mut tx = self.db.begin().await?;
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
        tx.commit().await?;
        Ok(kind)
    }

    async fn handle_unsubscribe(
        &self,
        event: UnsubscribeFeedRequested,
    ) -> EventConsumerResult<EventKind> {
        let mut tx = self.db.begin().await?;
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
        tx.commit().await?;
        Ok(kind)
    }
}

/// Public feed events projected from request and subscription facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiEventInput {
    events: Vec<Event>,
}

impl ApiEventInput {
    pub fn new(events: Vec<Event>) -> Self {
        Self { events }
    }

    pub fn into_events(self) -> Vec<Event> {
        self.events
    }
}

impl ConsumerEventInput for ApiEventInput {
    const READ_FILTER: EventReadFilter = EventReadFilter::new(&[
        EventKind::Request(RequestEventKind::SubscribeFeedRejected),
        EventKind::Request(RequestEventKind::UnsubscribeFeedRejected),
        EventKind::Sub(SubEventKind::FeedSubscribed),
        EventKind::Sub(SubEventKind::SubscriptionChanged),
        EventKind::Sub(SubEventKind::FeedUnsubscribed),
    ]);

    fn from_batch(batch: EventReadBatch) -> EventConsumerResult<Option<Self>> {
        let events = batch
            .into_events()
            .into_iter()
            .map(JournaledEvent::into_event)
            .collect::<Vec<_>>();
        Ok((!events.is_empty()).then_some(Self::new(events)))
    }
}

#[derive(Debug, Clone)]
pub struct ApiEventProj<S> {
    db: S,
}

impl<S> ApiEventProj<S> {
    pub fn new(db: S) -> Self {
        Self { db }
    }
}

impl<S> EventConsumer for ApiEventProj<S>
where
    S: FeedRegistryDb,
{
    type Input = ApiEventInput;

    fn id(&self) -> EventConsumerId {
        EventConsumerId::ApiEventProj
    }

    async fn consume(&mut self, input: Self::Input) -> EventConsumerResult<RecordedEvents> {
        let mut recorded = RecordedEvents::empty();
        let mut tx = self.db.begin().await?;
        for event in input.into_events() {
            let Some(api_event) = project_api_event(event) else {
                continue;
            };
            let event = Event::Api(api_event);
            recorded.push(event.kind());
            tx.append_event(event).await?;
        }
        tx.commit().await?;
        Ok(recorded)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiEventStreamInput {
    events: Vec<ApiEvent>,
}

impl ApiEventStreamInput {
    pub fn new(events: Vec<ApiEvent>) -> Self {
        Self { events }
    }

    pub fn into_events(self) -> Vec<ApiEvent> {
        self.events
    }
}

impl ConsumerEventInput for ApiEventStreamInput {
    const READ_FILTER: EventReadFilter = EventReadFilter::new(&[
        EventKind::Api(ApiEventKind::FeedSubscribed),
        EventKind::Api(ApiEventKind::FeedSubscribeRejected),
        EventKind::Api(ApiEventKind::FeedSubscriptionChanged),
        EventKind::Api(ApiEventKind::FeedUnsubscribed),
        EventKind::Api(ApiEventKind::FeedUnsubscribeRejected),
    ]);

    fn from_batch(batch: EventReadBatch) -> EventConsumerResult<Option<Self>> {
        let events = batch
            .into_events()
            .into_iter()
            .map(JournaledEvent::into_event)
            .map(|event| match event {
                Event::Api(event) => event,
                event => unreachable!("unexpected api event: {event:?}"),
            })
            .collect::<Vec<_>>();
        Ok((!events.is_empty()).then_some(Self::new(events)))
    }
}

#[derive(Debug, Clone)]
pub struct ApiEventStream {
    publisher: ApiEventPublisher,
}

impl ApiEventStream {
    pub fn new(publisher: ApiEventPublisher) -> Self {
        Self { publisher }
    }
}

impl EventConsumer for ApiEventStream {
    type Input = ApiEventStreamInput;

    fn id(&self) -> EventConsumerId {
        EventConsumerId::ApiEventStream
    }

    async fn consume(&mut self, input: Self::Input) -> EventConsumerResult<RecordedEvents> {
        for event in input.into_events() {
            let receivers = self.publisher.publish(event);
            debug!(receivers, "published feed event");
        }
        Ok(RecordedEvents::empty())
    }
}
