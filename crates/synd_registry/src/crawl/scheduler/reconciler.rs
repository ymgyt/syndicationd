use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tracing::debug;

use crate::{
    crawl::{
        schedule::ScheduleSync,
        scheduler::{
            driver::SchedDriver,
            input::{CrawlFinished, ManualRequested, SchedInput},
        },
    },
    db::{CrawlScheduleStore, FeedRegistryDb},
    event::{
        Event, EventInput, EventType, InputBatch, Processor, ProcessorError, ProcessorId,
        ProcessorResult, Reaction, Reconciler, WakeRequest,
    },
};

const DEFAULT_BATCH_SIZE: usize = 100;

pub struct CrawlReconciler {
    batch_size: usize,
    driver: SchedDriver,
}

impl CrawlReconciler {
    pub(crate) fn new(driver: SchedDriver) -> Self {
        Self {
            batch_size: DEFAULT_BATCH_SIZE,
            driver,
        }
    }
}

impl Processor for CrawlReconciler {
    type Input = CrawlReconcileInput;

    fn id(&self) -> ProcessorId {
        ProcessorId::CrawlReconciler
    }
}

impl<S> Reconciler<S> for CrawlReconciler
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlScheduleStore + Send,
{
    async fn reconcile(
        &mut self,
        tx: &mut S::Tx<'_>,
        now: DateTime<Utc>,
        _trigger: crate::event::Trigger,
        batch: InputBatch<Self::Input>,
    ) -> ProcessorResult<Reaction<Vec<Event>>> {
        let sync = ScheduleSync::new(now);
        let input_count = batch.len();
        let mut target_update_count = 0;
        let mut requested_count = 0;
        let mut schedule_upsert_count = 0;
        let mut finished_count = 0;

        for input in batch.into_inputs() {
            match input {
                CrawlReconcileInput::TargetUpdated(feed_url) => {
                    target_update_count += 1;
                    let Some(entry) = tx.load_schedule_sync_entry(&feed_url).await? else {
                        continue;
                    };
                    let Some(command) = sync.upsert_command(&entry) else {
                        continue;
                    };
                    tx.upsert_schedule(command).await?;
                    schedule_upsert_count += 1;
                }
                CrawlReconcileInput::CrawlFinished(input) => {
                    finished_count += 1;
                    if sync_crawl_finished_schedule(tx, &sync, &input).await? {
                        schedule_upsert_count += 1;
                    }
                    self.driver.submit(input.into());
                }
                CrawlReconcileInput::CrawlRequested(input) => {
                    requested_count += 1;
                    self.driver.submit(input.into());
                }
            }
        }

        debug!(
            input_count,
            target_update_count,
            requested_count,
            schedule_upsert_count,
            finished_count,
            "crawl reconcile events processed"
        );

        let scheduled_due = tx.list_scheduled_due(now, self.batch_size).await?;
        let scheduled_due_count = scheduled_due.len();
        self.driver
            .submit_batch(scheduled_due.into_iter().map(SchedInput::from));
        debug!(scheduled_due_count, "scheduled crawl due submitted");
        let wake_request = tx
            .next_scheduled_due(now)
            .await?
            .map(WakeRequest::at)
            .unwrap_or(WakeRequest::None);

        match self.driver.dispatch(now) {
            Ok(dispatched_count) => {
                debug!(dispatched_count, "crawl dispatch queue updated");
            }
            Err(err) => {
                debug!(error = ?err, "crawl dispatch queue update skipped");
            }
        }
        Ok(Reaction::new(Vec::new(), wake_request))
    }
}

async fn sync_crawl_finished_schedule<Tx>(
    tx: &mut Tx,
    sync: &ScheduleSync,
    input: &CrawlFinished,
) -> ProcessorResult<bool>
where
    Tx: CrawlScheduleStore + Send,
{
    let Some(entry) = tx.load_schedule_sync_entry(&input.feed_url).await? else {
        return Ok(false);
    };
    let Some(command) = sync.crawl_finished_command(&entry, input.finished_at) else {
        return Ok(false);
    };
    tx.upsert_schedule(command).await?;
    Ok(true)
}

/// Journal event consumed by the crawl reconciler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrawlReconcileInput {
    TargetUpdated(FeedUrl),
    CrawlRequested(ManualRequested),
    CrawlFinished(CrawlFinished),
}

impl EventInput for CrawlReconcileInput {
    const INTERESTS: &'static [EventType] = &[
        EventType::CrawlTargetActivated,
        EventType::CrawlTargetPolicyChanged,
        EventType::CrawlTargetDeactivated,
        EventType::CrawlRequested,
        EventType::CrawlJobFinished,
    ];

    fn from_event(event: Event, occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::CrawlTargetActivated(event) => Ok(Self::TargetUpdated(event.feed_url)),
            Event::CrawlTargetPolicyChanged(event) => Ok(Self::TargetUpdated(event.feed_url)),
            Event::CrawlTargetDeactivated(event) => Ok(Self::TargetUpdated(event.feed_url)),
            Event::CrawlRequested(event) => Ok(Self::CrawlRequested(ManualRequested {
                feed_url: event.feed_url,
                requested_at: occurred_at,
            })),
            Event::CrawlJobFinished(event) => Ok(Self::CrawlFinished(CrawlFinished::new(
                event.feed_url,
                occurred_at,
            ))),
            event => Err(ProcessorError::unexpected_input(
                "crawl reconcile event",
                &event,
            )),
        }
    }
}
