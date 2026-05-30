use crate::event::{
    ConsumerEventInput, Event, EventConsumer, EventConsumerId, EventConsumerResult,
    EventConsumerSession, EventJournal, EventKind, EventReadBatch, EventReadFilter, JournaledEvent,
    SubEvent, SubEventKind, SubscriptionLifecycle,
};

/// Subscription lifecycle events relevant to the crawl target list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlTargetListInput {
    events: Vec<SubscriptionLifecycle>,
}

impl CrawlTargetListInput {
    pub fn new(events: Vec<SubscriptionLifecycle>) -> Self {
        Self { events }
    }

    pub fn into_events(self) -> Vec<SubscriptionLifecycle> {
        self.events
    }
}

impl ConsumerEventInput for CrawlTargetListInput {
    const READ_FILTER: EventReadFilter = EventReadFilter::new(&[
        EventKind::Sub(SubEventKind::FeedSubscribed),
        EventKind::Sub(SubEventKind::FeedUnsubscribed),
    ]);

    fn from_batch(batch: EventReadBatch) -> EventConsumerResult<Option<Self>> {
        let events = batch
            .into_events()
            .into_iter()
            .map(JournaledEvent::into_event)
            .map(|event| match event {
                Event::Sub(SubEvent::FeedSubscribed(event)) => {
                    SubscriptionLifecycle::Subscribed(event)
                }
                Event::Sub(SubEvent::FeedUnsubscribed(event)) => {
                    SubscriptionLifecycle::Unsubscribed(event)
                }
                event => unreachable!("unexpected crawl target list event: {event:?}"),
            })
            .collect::<Vec<_>>();

        Ok((!events.is_empty()).then_some(Self::new(events)))
    }
}

/// Consumer that reacts to subscription events for the crawl target list.
#[derive(Debug, Default, Clone, Copy)]
pub struct CrawlTargetListProj;

impl CrawlTargetListProj {
    pub fn new() -> Self {
        Self
    }
}

impl EventConsumer for CrawlTargetListProj {
    type Input = CrawlTargetListInput;

    fn id(&self) -> EventConsumerId {
        EventConsumerId::CrawlTargetListProj
    }

    async fn consume<J>(
        &mut self,
        input: Self::Input,
        _session: &mut EventConsumerSession<'_, J>,
    ) -> EventConsumerResult<()>
    where
        J: EventJournal,
    {
        let event_count = input.into_events().len();
        tracing::debug!(event_count, "crawl target list projector received events");
        Ok(())
    }
}
