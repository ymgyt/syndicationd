use crate::event::{
    ConsumerEventInput, EventConsumer, EventConsumerId, EventConsumerResult, EventConsumerSession,
    EventJournal, EventReadBatch, EventReadFilter, JournaledEvent, RegistryEvent,
    RegistryEventKind, SubscriptionLifecycle,
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
        RegistryEventKind::FeedSubscribed,
        RegistryEventKind::FeedUnsubscribed,
    ]);

    fn from_batch(batch: EventReadBatch) -> EventConsumerResult<Option<Self>> {
        let events = batch
            .into_events()
            .into_iter()
            .map(JournaledEvent::into_event)
            .map(|event| match event {
                RegistryEvent::SubscriptionLifecycle(event) => event,
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
        let _events = input.into_events();
        Ok(())
    }
}
