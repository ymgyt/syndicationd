use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;

use crate::{
    db::{FeedRegistryDb, TimelineProjectionTx},
    error::RegistryDbResult,
    event::{
        ConsumeContext, Consumer, Event, EventInterests, FeedSubscribedEvent, InputBatch,
        JournalTx, Processor, ProcessorError, ProcessorId, ProcessorResult, RecordedEvents,
        SubEvent, SubEventKind, TimelineChangedEvent, Transactional,
    },
    timeline::{TimelineCatchup, TimelineKey},
};

/// Event input used to project timeline state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineProjectionInput {
    FeedSubscribed(FeedSubscribedEvent),
}

impl TryFrom<Event> for TimelineProjectionInput {
    type Error = ProcessorError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Sub(SubEvent::FeedSubscribed(event)) => Ok(Self::FeedSubscribed(event)),
            event => Err(ProcessorError::UnexpectedEvent {
                expected: "timeline projection event",
                actual: event.kind(),
            }),
        }
    }
}

/// Projects subscription facts into timeline membership.
#[derive(Debug, Clone)]
pub struct TimelineProj;

impl TimelineProj {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TimelineProj {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for TimelineProj {
    type Input = TimelineProjectionInput;
    type Phase = Transactional;

    fn id(&self) -> ProcessorId {
        ProcessorId::TimelineProjection
    }

    fn interests(&self) -> EventInterests {
        EventInterests::new([SubEventKind::FeedSubscribed.into()])
    }
}

impl<S> Consumer<S> for TimelineProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: TimelineProjectionTx + JournalTx + Send,
{
    async fn consume(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        input: Self::Input,
    ) -> ProcessorResult<()> {
        <Self as Consumer<S>>::consume_batch(self, cx, InputBatch::new(vec![input])).await
    }

    async fn consume_batch(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        batch: InputBatch<Self::Input>,
    ) -> ProcessorResult<()> {
        let now = Utc::now();
        let mut invalidations = TimelineInvalidations::empty();
        let mut scope = cx.timeline_projection();

        for input in unique_inputs(batch.into_inputs()) {
            match input {
                TimelineProjectionInput::FeedSubscribed(event) => {
                    let subscription = event.subscription;
                    let timeline = TimelineKey::default_for(subscription.subscriber_id);
                    let catchup = scope
                        .catchup_default_timeline_feed(&timeline, &subscription.feed_url, now)
                        .await?;
                    if catchup.inserted_items() > 0 {
                        invalidations.mark(catchup.timeline().clone(), catchup.feed_url().clone());
                    }
                }
            }
        }

        scope
            .record_timeline_invalidations(invalidations, now)
            .await?;
        Ok(())
    }
}

/// Transaction-scoped operations for projecting timeline membership.
pub struct TimelineProjectionScope<'a, Tx> {
    tx: &'a mut Tx,
    recorded: &'a mut RecordedEvents,
}

impl<'a, Tx> TimelineProjectionScope<'a, Tx> {
    pub fn new(tx: &'a mut Tx, recorded: &'a mut RecordedEvents) -> Self {
        Self { tx, recorded }
    }
}

impl<Tx> TimelineProjectionScope<'_, Tx>
where
    Tx: TimelineProjectionTx + JournalTx + Send,
{
    pub async fn catchup_default_timeline_feed(
        &mut self,
        timeline: &TimelineKey,
        feed_url: &FeedUrl,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<TimelineCatchup> {
        self.tx.ensure_default_timeline(timeline, now).await?;
        self.tx.catchup_timeline_feed(timeline, feed_url, now).await
    }

    async fn record_timeline_invalidations(
        &mut self,
        invalidations: TimelineInvalidations,
        changed_at: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        for event in invalidations.into_events(changed_at) {
            self.record_event(event).await?;
        }
        Ok(())
    }

    async fn record_event<E>(&mut self, event: E) -> RegistryDbResult<()>
    where
        E: Into<Event>,
    {
        let event = event.into();
        let kind = event.kind();
        self.tx.append_event(event).await?;
        self.recorded.push(kind);
        Ok(())
    }
}

fn unique_inputs(inputs: Vec<TimelineProjectionInput>) -> Vec<TimelineProjectionInput> {
    let mut unique = Vec::with_capacity(inputs.len());
    for input in inputs {
        if unique.iter().any(|seen| same_input(seen, &input)) {
            continue;
        }
        unique.push(input);
    }
    unique
}

fn same_input(a: &TimelineProjectionInput, b: &TimelineProjectionInput) -> bool {
    match (a, b) {
        (
            TimelineProjectionInput::FeedSubscribed(a),
            TimelineProjectionInput::FeedSubscribed(b),
        ) => a.subscription == b.subscription,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TimelineInvalidations {
    changes: Vec<TimelineInvalidation>,
}

impl TimelineInvalidations {
    fn empty() -> Self {
        Self {
            changes: Vec::new(),
        }
    }

    fn mark(&mut self, timeline: TimelineKey, feed_url: FeedUrl) {
        if let Some(change) = self
            .changes
            .iter_mut()
            .find(|change| change.timeline == timeline)
        {
            if !change.affected_feeds.contains(&feed_url) {
                change.affected_feeds.push(feed_url);
            }
            return;
        }
        self.changes.push(TimelineInvalidation {
            timeline,
            affected_feeds: vec![feed_url],
        });
    }

    fn into_events(self, changed_at: DateTime<Utc>) -> Vec<TimelineChangedEvent> {
        self.changes
            .into_iter()
            .map(|change| {
                TimelineChangedEvent::new(change.timeline, changed_at, change.affected_feeds)
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TimelineInvalidation {
    timeline: TimelineKey,
    affected_feeds: Vec<FeedUrl>,
}
