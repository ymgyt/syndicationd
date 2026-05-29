use crate::{
    crawl::target_list::CrawlTargetListProj,
    event::{
        ConsumerDispatch, ConsumerEventInput, ConsumerRegistry, EventConsumer, EventConsumerId,
        EventConsumerResult, EventConsumerSession, EventJournal, EventReadBatch, EventReadFilter,
    },
};

const CONSUMER_IDS: &[EventConsumerId] = &[EventConsumerId::CrawlTargetListProj];

/// One registered consumer selected for a journal batch.
#[derive(Debug, Clone, Copy)]
pub enum RegisteredConsumer {
    CrawlTargetListProj(CrawlTargetListProj),
}

impl ConsumerDispatch for RegisteredConsumer {
    async fn consume<J>(
        self,
        batch: EventReadBatch,
        session: &mut EventConsumerSession<'_, J>,
    ) -> EventConsumerResult<()>
    where
        J: EventJournal,
    {
        match self {
            Self::CrawlTargetListProj(mut consumer) => {
                let Some(input) = <CrawlTargetListProj as EventConsumer>::Input::from_batch(batch)?
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
pub struct Consumers {
    crawl_target_list_proj: CrawlTargetListProj,
}

impl Consumers {
    pub fn new(crawl_target_list_proj: CrawlTargetListProj) -> Self {
        Self {
            crawl_target_list_proj,
        }
    }
}

impl ConsumerRegistry for Consumers {
    type Dispatch<'a> = RegisteredConsumer;

    fn ids(&self) -> &'static [EventConsumerId] {
        CONSUMER_IDS
    }

    fn read_filter(&self, id: EventConsumerId) -> Option<EventReadFilter> {
        match id {
            EventConsumerId::CrawlTargetListProj => Some(self.crawl_target_list_proj.read_filter()),
            _ => None,
        }
    }

    fn dispatch(&self, id: EventConsumerId) -> Option<Self::Dispatch<'_>> {
        match id {
            EventConsumerId::CrawlTargetListProj => Some(RegisteredConsumer::CrawlTargetListProj(
                self.crawl_target_list_proj,
            )),
            _ => None,
        }
    }
}
