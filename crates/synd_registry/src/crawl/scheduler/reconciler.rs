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
        ProcessorResult, Reaction, Reconciler, Trigger, WakeRequest,
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
        trigger: Trigger,
        batch: InputBatch<Self::Input>,
    ) -> ProcessorResult<Reaction<Vec<Event>>> {
        let mut ctx = ReconcileCtx::new(now, trigger, batch.len());

        self.reconcile_inputs(tx, &mut ctx, batch).await?;
        ctx.log_events_processed();

        self.submit_scheduled_due(tx, &mut ctx).await?;
        ctx.log_scheduled_due_submitted();

        self.request_next_wake(tx, &mut ctx).await?;
        self.dispatch(&ctx);

        Ok(Reaction::new(Vec::new(), ctx.wake_request()))
    }
}

impl CrawlReconciler {
    async fn reconcile_inputs<Tx>(
        &mut self,
        tx: &mut Tx,
        ctx: &mut ReconcileCtx,
        batch: InputBatch<CrawlReconcileInput>,
    ) -> ProcessorResult<()>
    where
        Tx: CrawlScheduleStore + Send,
    {
        for input in batch.into_inputs() {
            match input {
                CrawlReconcileInput::TargetUpdated(feed_url) => {
                    ctx.record_target_updated();
                    if sync_target_schedule(tx, ctx.sync(), &feed_url).await? {
                        ctx.record_schedule_upserted();
                    }
                }
                CrawlReconcileInput::CrawlFinished(input) => {
                    ctx.record_finished();
                    if sync_crawl_finished_schedule(tx, ctx.sync(), &input).await? {
                        ctx.record_schedule_upserted();
                    }
                    self.driver.submit(input.into());
                }
                CrawlReconcileInput::CrawlRequested(input) => {
                    ctx.record_requested();
                    self.driver.submit(input.into());
                }
            }
        }
        Ok(())
    }

    async fn submit_scheduled_due<Tx>(
        &mut self,
        tx: &mut Tx,
        ctx: &mut ReconcileCtx,
    ) -> ProcessorResult<()>
    where
        Tx: CrawlScheduleStore + Send,
    {
        let scheduled_due = tx.list_scheduled_due(ctx.now(), self.batch_size).await?;
        ctx.set_scheduled_due_count(scheduled_due.len());
        self.driver
            .submit_batch(scheduled_due.into_iter().map(SchedInput::from));
        Ok(())
    }

    async fn request_next_wake<Tx>(
        &mut self,
        tx: &mut Tx,
        ctx: &mut ReconcileCtx,
    ) -> ProcessorResult<()>
    where
        Tx: CrawlScheduleStore + Send,
    {
        let wake_request = tx
            .next_scheduled_due(ctx.now())
            .await?
            .map_or(WakeRequest::None, WakeRequest::at);
        ctx.set_wake_request(wake_request);
        Ok(())
    }

    fn dispatch(&mut self, ctx: &ReconcileCtx) {
        match self.driver.dispatch(ctx.now()) {
            Ok(dispatched_count) => {
                debug!(dispatched_count, "crawl dispatch queue updated");
            }
            Err(err) => {
                debug!(error = ?err, "crawl dispatch queue update skipped");
            }
        }
    }
}

struct ReconcileCtx {
    now: DateTime<Utc>,
    trigger: Trigger,
    sync: ScheduleSync,
    input_count: usize,
    target_updated_count: usize,
    requested_count: usize,
    schedule_upsert_count: usize,
    finished_count: usize,
    scheduled_due_count: usize,
    wake_request: WakeRequest,
}

impl ReconcileCtx {
    fn new(now: DateTime<Utc>, trigger: Trigger, input_count: usize) -> Self {
        Self {
            now,
            trigger,
            sync: ScheduleSync::new(now),
            input_count,
            target_updated_count: 0,
            requested_count: 0,
            schedule_upsert_count: 0,
            finished_count: 0,
            scheduled_due_count: 0,
            wake_request: WakeRequest::None,
        }
    }

    fn now(&self) -> DateTime<Utc> {
        self.now
    }

    fn sync(&self) -> &ScheduleSync {
        &self.sync
    }

    fn wake_request(&self) -> WakeRequest {
        self.wake_request
    }

    fn record_target_updated(&mut self) {
        self.target_updated_count += 1;
    }

    fn record_requested(&mut self) {
        self.requested_count += 1;
    }

    fn record_finished(&mut self) {
        self.finished_count += 1;
    }

    fn record_schedule_upserted(&mut self) {
        self.schedule_upsert_count += 1;
    }

    fn set_scheduled_due_count(&mut self, count: usize) {
        self.scheduled_due_count = count;
    }

    fn set_wake_request(&mut self, wake_request: WakeRequest) {
        self.wake_request = wake_request;
    }

    fn log_events_processed(&self) {
        debug!(
            trigger = self.trigger.as_str(),
            input_count = self.input_count,
            target_updated_count = self.target_updated_count,
            requested_count = self.requested_count,
            schedule_upsert_count = self.schedule_upsert_count,
            finished_count = self.finished_count,
            "crawl reconcile events processed"
        );
    }

    fn log_scheduled_due_submitted(&self) {
        debug!(
            trigger = self.trigger.as_str(),
            scheduled_due_count = self.scheduled_due_count,
            "scheduled crawl due submitted"
        );
    }
}

async fn sync_target_schedule<Tx>(
    tx: &mut Tx,
    sync: &ScheduleSync,
    feed_url: &FeedUrl,
) -> ProcessorResult<bool>
where
    Tx: CrawlScheduleStore + Send,
{
    let Some(entry) = tx.load_schedule_sync_entry(feed_url).await? else {
        return Ok(false);
    };
    let Some(command) = sync.upsert_command(&entry) else {
        return Ok(false);
    };
    tx.upsert_schedule(command).await?;
    Ok(true)
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
