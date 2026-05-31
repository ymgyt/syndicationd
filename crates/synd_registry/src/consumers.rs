use chrono::Utc;
use tracing::debug;

use crate::{
    crawl::target_list::CrawlTargetListProj,
    db::{FeedRegistryDb, RegistryDbTransaction},
    event::{
        ApiEvent, ApiEventKind, ApiEventPublisher, ApiFeedSubscribeRejected, ApiFeedSubscribed,
        ApiFeedSubscriptionChanged, ApiFeedUnsubscribeRejected, ApiFeedUnsubscribed,
        ConsumerDispatch, ConsumerEventInput, ConsumerRegistry, Event, EventConsumer,
        EventConsumerError, EventConsumerId, EventConsumerResult, EventConsumerSession,
        EventJournal, EventKind, EventReadBatch, EventReadFilter, FeedSubscribed, FeedUnsubscribed,
        JournaledEvent, RequestEvent, RequestEventKind, SubEvent, SubEventKind,
        SubscribeFeedRequested, SubscriptionChanged, UnsubscribeFeedRejected,
        UnsubscribeFeedRequested,
    },
    subscription::Subscription,
};

const CONSUMER_IDS: &[EventConsumerId] = &[
    EventConsumerId::SubRequestWorker,
    EventConsumerId::CrawlTargetListProj,
    EventConsumerId::ApiEventProj,
    EventConsumerId::ApiEventStream,
];

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

    async fn consume<J>(
        &mut self,
        input: Self::Input,
        session: &mut EventConsumerSession<'_, J>,
    ) -> EventConsumerResult<()>
    where
        J: EventJournal,
    {
        for event in input.into_events() {
            match event {
                RequestEvent::SubscribeFeedRequested(event) => {
                    self.handle_subscribe(event, session).await?;
                }
                RequestEvent::UnsubscribeFeedRequested(event) => {
                    self.handle_unsubscribe(event, session).await?;
                }
                event => unreachable!("unexpected subscription request event: {event:?}"),
            }
        }
        Ok(())
    }
}

impl<S> SubRequestWorker<S>
where
    S: FeedRegistryDb,
{
    async fn handle_subscribe<J>(
        &self,
        event: SubscribeFeedRequested,
        session: &mut EventConsumerSession<'_, J>,
    ) -> EventConsumerResult<()>
    where
        J: EventJournal,
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

        let mut tx = self.db.begin().await.map_err(consumer_error)?;
        let already_subscribed = tx
            .has_subscription(&subscription.subscriber_id, &subscription.feed_url)
            .await
            .map_err(consumer_error)?;
        tx.upsert_subscription(subscription)
            .await
            .map_err(consumer_error)?;

        let event = if already_subscribed {
            Event::Sub(SubEvent::SubscriptionChanged(
                SubscriptionChanged::new(event.subscription).with_request_id(event.request_id),
            ))
        } else {
            Event::Sub(SubEvent::FeedSubscribed(
                FeedSubscribed::new(event.subscription).with_request_id(event.request_id),
            ))
        };
        tx.append_event(event.clone())
            .await
            .map_err(consumer_error)?;
        tx.commit().await.map_err(consumer_error)?;
        session.record_committed(&event);
        Ok(())
    }

    async fn handle_unsubscribe<J>(
        &self,
        event: UnsubscribeFeedRequested,
        session: &mut EventConsumerSession<'_, J>,
    ) -> EventConsumerResult<()>
    where
        J: EventJournal,
    {
        let mut tx = self.db.begin().await.map_err(consumer_error)?;
        let is_subscribed = tx
            .has_subscription(
                &event.subscription.subscriber_id,
                &event.subscription.feed_url,
            )
            .await
            .map_err(consumer_error)?;

        let event = if is_subscribed {
            tx.delete_subscription(
                &event.subscription.subscriber_id,
                &event.subscription.feed_url,
            )
            .await
            .map_err(consumer_error)?;
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

        tx.append_event(event.clone())
            .await
            .map_err(consumer_error)?;
        tx.commit().await.map_err(consumer_error)?;
        session.record_committed(&event);
        Ok(())
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

#[derive(Debug, Default, Clone, Copy)]
pub struct ApiEventProj;

impl ApiEventProj {
    pub fn new() -> Self {
        Self
    }
}

impl EventConsumer for ApiEventProj {
    type Input = ApiEventInput;

    fn id(&self) -> EventConsumerId {
        EventConsumerId::ApiEventProj
    }

    async fn consume<J>(
        &mut self,
        input: Self::Input,
        session: &mut EventConsumerSession<'_, J>,
    ) -> EventConsumerResult<()>
    where
        J: EventJournal,
    {
        for event in input.into_events() {
            let Some(api_event) = project_api_event(event) else {
                continue;
            };
            session.record(Event::Api(api_event)).await?;
        }
        Ok(())
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

    async fn consume<J>(
        &mut self,
        input: Self::Input,
        _session: &mut EventConsumerSession<'_, J>,
    ) -> EventConsumerResult<()>
    where
        J: EventJournal,
    {
        for event in input.into_events() {
            let receivers = self.publisher.publish(event);
            debug!(receivers, "published feed event");
        }
        Ok(())
    }
}

/// One registered consumer selected for a journal batch.
#[derive(Debug, Clone)]
pub enum RegisteredConsumer<S> {
    SubRequestWorker(SubRequestWorker<S>),
    CrawlTargetListProj(CrawlTargetListProj<S>),
    ApiEventProj(ApiEventProj),
    ApiEventStream(ApiEventStream),
}

impl<S> ConsumerDispatch for RegisteredConsumer<S>
where
    S: FeedRegistryDb,
{
    async fn consume<J>(
        self,
        batch: EventReadBatch,
        session: &mut EventConsumerSession<'_, J>,
    ) -> EventConsumerResult<()>
    where
        J: EventJournal,
    {
        match self {
            Self::SubRequestWorker(mut consumer) => {
                let Some(input) = <SubRequestWorker<S> as EventConsumer>::Input::from_batch(batch)?
                else {
                    return Ok(());
                };
                consumer.consume(input, session).await
            }
            Self::CrawlTargetListProj(mut consumer) => {
                let Some(input) =
                    <CrawlTargetListProj<S> as EventConsumer>::Input::from_batch(batch)?
                else {
                    return Ok(());
                };
                consumer.consume(input, session).await
            }
            Self::ApiEventProj(mut consumer) => {
                let Some(input) = <ApiEventProj as EventConsumer>::Input::from_batch(batch)? else {
                    return Ok(());
                };
                consumer.consume(input, session).await
            }
            Self::ApiEventStream(mut consumer) => {
                let Some(input) = <ApiEventStream as EventConsumer>::Input::from_batch(batch)?
                else {
                    return Ok(());
                };
                consumer.consume(input, session).await
            }
        }
    }
}

/// Concrete event consumers used by the registry event runtime.
#[derive(Debug, Clone)]
pub struct Consumers<S> {
    sub_request_worker: SubRequestWorker<S>,
    crawl_target_list_proj: CrawlTargetListProj<S>,
    api_event_proj: ApiEventProj,
    api_event_stream: ApiEventStream,
}

impl<S> Consumers<S> {
    pub fn new(
        sub_request_worker: SubRequestWorker<S>,
        crawl_target_list_proj: CrawlTargetListProj<S>,
        api_event_proj: ApiEventProj,
        api_event_stream: ApiEventStream,
    ) -> Self {
        Self {
            sub_request_worker,
            crawl_target_list_proj,
            api_event_proj,
            api_event_stream,
        }
    }
}

impl<S> ConsumerRegistry for Consumers<S>
where
    S: FeedRegistryDb,
{
    type Dispatch<'a>
        = RegisteredConsumer<S>
    where
        Self: 'a;

    fn ids(&self) -> &'static [EventConsumerId] {
        CONSUMER_IDS
    }

    fn read_filter(&self, id: EventConsumerId) -> Option<EventReadFilter> {
        match id {
            EventConsumerId::SubRequestWorker => Some(self.sub_request_worker.read_filter()),
            EventConsumerId::CrawlTargetListProj => Some(self.crawl_target_list_proj.read_filter()),
            EventConsumerId::ApiEventProj => Some(self.api_event_proj.read_filter()),
            EventConsumerId::ApiEventStream => Some(self.api_event_stream.read_filter()),
            _ => None,
        }
    }

    fn dispatch(&self, id: EventConsumerId) -> Option<Self::Dispatch<'_>> {
        match id {
            EventConsumerId::SubRequestWorker => Some(RegisteredConsumer::SubRequestWorker(
                self.sub_request_worker.clone(),
            )),
            EventConsumerId::CrawlTargetListProj => Some(RegisteredConsumer::CrawlTargetListProj(
                self.crawl_target_list_proj.clone(),
            )),
            EventConsumerId::ApiEventProj => {
                Some(RegisteredConsumer::ApiEventProj(self.api_event_proj))
            }
            EventConsumerId::ApiEventStream => Some(RegisteredConsumer::ApiEventStream(
                self.api_event_stream.clone(),
            )),
            _ => None,
        }
    }
}

fn consumer_error(err: impl Into<anyhow::Error>) -> EventConsumerError {
    EventConsumerError::Internal(err.into())
}
