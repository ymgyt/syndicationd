use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tracing::debug;

use crate::{
    crawl::schedule::{
        CompleteDispatchCommand, DueReason, ScheduleSync, UpsertCrawlScheduleCommand,
    },
    db::{CrawlResultStore, CrawlScheduleStore, FeedRegistryDb},
    event::{
        CrawlJobFinishedEvent, CrawlRequestedEvent, CrawlScheduleUpdatedEvent,
        CrawlTargetActivatedEvent, CrawlTargetDeactivatedEvent, CrawlTargetPolicyChangedEvent,
        Event, EventInput, EventType, Processor, ProcessorError, ProcessorId, ProcessorResult,
        Projector, RegistryEvent,
    },
};

/// Journal event consumed by the crawl schedule projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrawlScheduleProjInput {
    TargetUpdated {
        feed_url: FeedUrl,
        occurred_at: DateTime<Utc>,
    },
    ManualRequested {
        feed_url: FeedUrl,
        requested_at: DateTime<Utc>,
    },
    CrawlFinished {
        feed_url: FeedUrl,
        finished_at: DateTime<Utc>,
    },
}

impl EventInput for CrawlScheduleProjInput {
    const INTERESTS: &'static [EventType] = &[
        CrawlTargetActivatedEvent::TYPE,
        CrawlTargetPolicyChangedEvent::TYPE,
        CrawlTargetDeactivatedEvent::TYPE,
        CrawlRequestedEvent::TYPE,
        CrawlJobFinishedEvent::TYPE,
    ];

    fn from_event(event: Event, occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::CrawlTargetActivated(event) => Ok(Self::TargetUpdated {
                feed_url: event.feed_url,
                occurred_at,
            }),
            Event::CrawlTargetPolicyChanged(event) => Ok(Self::TargetUpdated {
                feed_url: event.feed_url,
                occurred_at,
            }),
            Event::CrawlTargetDeactivated(event) => Ok(Self::TargetUpdated {
                feed_url: event.feed_url,
                occurred_at,
            }),
            Event::CrawlRequested(event) => Ok(Self::ManualRequested {
                feed_url: event.feed_url,
                requested_at: occurred_at,
            }),
            Event::CrawlJobFinished(event) => Ok(Self::CrawlFinished {
                feed_url: event.feed_url,
                finished_at: occurred_at,
            }),
            event => Err(ProcessorError::unexpected_input(
                "crawl schedule event",
                &event,
            )),
        }
    }
}

/// Projects crawl target and crawl completion facts into `crawl_schedule` rows.
#[derive(Debug, Clone)]
pub struct CrawlScheduleProj;

impl CrawlScheduleProj {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CrawlScheduleProj {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for CrawlScheduleProj {
    type Input = CrawlScheduleProjInput;

    fn id(&self) -> ProcessorId {
        ProcessorId::CrawlScheduleProjection
    }
}

impl CrawlScheduleProjInput {
    fn feed_url(&self) -> &FeedUrl {
        match self {
            Self::TargetUpdated { feed_url, .. }
            | Self::ManualRequested { feed_url, .. }
            | Self::CrawlFinished { feed_url, .. } => feed_url,
        }
    }
}

/// The single schedule write decided for one projected input.
enum ScheduleWrite {
    Upsert(UpsertCrawlScheduleCommand),
    CompleteDispatch(CompleteDispatchCommand),
}

impl ScheduleWrite {
    fn next_crawl_after(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Upsert(command) => command.next_crawl_after,
            Self::CompleteDispatch(command) => command.next_crawl_after,
        }
    }

    fn due_reason(&self) -> DueReason {
        match self {
            Self::Upsert(command) => command.due_reason,
            Self::CompleteDispatch(command) => command.due_reason,
        }
    }

    async fn apply<Tx>(self, tx: &mut Tx) -> ProcessorResult<()>
    where
        Tx: CrawlScheduleStore + Send,
    {
        match self {
            Self::Upsert(command) => tx.upsert_schedule(command).await?,
            Self::CompleteDispatch(command) => tx.complete_dispatch(command).await?,
        }
        Ok(())
    }
}

impl<S> Projector<S> for CrawlScheduleProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlScheduleStore + CrawlResultStore + Send,
{
    async fn project(
        &mut self,
        tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        let feed_url = input.feed_url().clone();
        // observe
        let Some(entry) = tx.load_schedule_sync_entry(&feed_url).await? else {
            return Ok(Vec::new());
        };

        // decide
        let write = match input {
            CrawlScheduleProjInput::TargetUpdated { occurred_at, .. } => {
                ScheduleSync::new(occurred_at)
                    .upsert_command(&entry)
                    .map(ScheduleWrite::Upsert)
            }
            CrawlScheduleProjInput::ManualRequested { requested_at, .. } => {
                ScheduleSync::new(requested_at)
                    .manual_request_command(&entry, requested_at)
                    .map(ScheduleWrite::Upsert)
            }
            CrawlScheduleProjInput::CrawlFinished { finished_at, .. } => {
                let crawl_state = tx.load_crawl_state(&feed_url).await?;
                Some(ScheduleWrite::CompleteDispatch(
                    ScheduleSync::new(finished_at).crawl_finished_command(
                        &entry,
                        crawl_state.as_ref(),
                        finished_at,
                    ),
                ))
            }
        };
        let Some(write) = write else {
            return Ok(Vec::new());
        };

        // apply
        debug!(
            feed_url = feed_url.as_str(),
            next_crawl_after = ?write.next_crawl_after(),
            due_reason = write.due_reason().as_str(),
            "crawl schedule updated"
        );
        write.apply(tx).await?;
        Ok(vec![CrawlScheduleUpdatedEvent::new(feed_url).into()])
    }
}
