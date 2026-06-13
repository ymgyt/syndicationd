use chrono::{DateTime, Utc};
use synd_feed::types::{EntryId, FeedUrl};

use crate::{
    db::{FeedRegistryDb, TimelineTx},
    error::RegistryDbResult,
    event::{
        ConsumeContext, Consumer, ConsumerInput, EntryChangedEvent, EntryDiscoveredEvent, Event,
        EventType, FeedSubscribedEvent, FeedUnsubscribedEvent, InputBatch, Processor,
        ProcessorError, ProcessorId, ProcessorResult, RegistryEvent, TimelineChangedEvent,
        skip_permanent_error,
    },
    subscription::SubscriptionKey,
    timeline::{TimelineCatchup, TimelineKey},
};

/// Event input used to project timeline state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineProjectionInput {
    FeedSubscribed {
        event: FeedSubscribedEvent,
        occurred_at: DateTime<Utc>,
    },
    FeedUnsubscribed {
        event: FeedUnsubscribedEvent,
        occurred_at: DateTime<Utc>,
    },
    EntryDiscovered {
        event: EntryDiscoveredEvent,
        occurred_at: DateTime<Utc>,
    },
    EntryChanged {
        event: EntryChangedEvent,
        occurred_at: DateTime<Utc>,
    },
}

impl TimelineProjectionInput {
    fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::FeedSubscribed { occurred_at, .. }
            | Self::FeedUnsubscribed { occurred_at, .. }
            | Self::EntryDiscovered { occurred_at, .. }
            | Self::EntryChanged { occurred_at, .. } => *occurred_at,
        }
    }
}

impl ConsumerInput for TimelineProjectionInput {
    const INTERESTS: &'static [EventType] = &[
        FeedSubscribedEvent::TYPE,
        FeedUnsubscribedEvent::TYPE,
        EntryDiscoveredEvent::TYPE,
        EntryChangedEvent::TYPE,
    ];

    fn from_event(event: Event, occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::FeedSubscribed(event) => Ok(Self::FeedSubscribed { event, occurred_at }),
            Event::FeedUnsubscribed(event) => Ok(Self::FeedUnsubscribed { event, occurred_at }),
            Event::EntryDiscovered(event) => Ok(Self::EntryDiscovered { event, occurred_at }),
            Event::EntryChanged(event) => Ok(Self::EntryChanged { event, occurred_at }),
            event => Err(ProcessorError::unexpected_input(
                "timeline projection event",
                &event,
            )),
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

    fn id(&self) -> ProcessorId {
        ProcessorId::TimelineProjection
    }
}

impl<S> Consumer<S> for TimelineProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: TimelineTx + Send,
{
    async fn consume(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        <Self as Consumer<S>>::consume_batch(self, cx, InputBatch::new(vec![input])).await
    }

    async fn consume_batch(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        batch: InputBatch<Self::Input>,
    ) -> ProcessorResult<Vec<Event>> {
        let processor = self.id();
        let mut invalidations = TimelineInvalidations::empty();
        let mut scope = cx.timeline_projection();

        for input in batch.into_inputs() {
            let occurred_at = input.occurred_at();
            let consumed = match input {
                TimelineProjectionInput::FeedSubscribed { event, .. } => {
                    let subscription = event.subscription;
                    let timeline = TimelineKey::default_for(subscription.subscriber_id);
                    match scope
                        .catchup_default_timeline_feed(
                            &timeline,
                            &subscription.feed_url,
                            occurred_at,
                        )
                        .await
                    {
                        Ok(catchup) => {
                            if catchup.inserted_items() > 0 {
                                invalidations.mark(
                                    catchup.timeline().clone(),
                                    catchup.feed_url().clone(),
                                    occurred_at,
                                );
                            }
                            Ok(())
                        }
                        Err(err) => Err(err.into()),
                    }
                }
                TimelineProjectionInput::FeedUnsubscribed { event, .. } => {
                    let subscription = event.subscription;
                    match scope.apply_feed_unsubscribed(&subscription).await {
                        Ok(timeline) => {
                            if let Some(timeline) = timeline {
                                invalidations.mark(timeline, subscription.feed_url, occurred_at);
                            }
                            Ok(())
                        }
                        Err(err) => Err(err.into()),
                    }
                }
                TimelineProjectionInput::EntryDiscovered { event, .. } => {
                    match scope
                        .apply_entry_discovered(&event.feed_url, &event.entry_id, occurred_at)
                        .await
                    {
                        Ok(timelines) => {
                            for timeline in timelines {
                                invalidations.mark(timeline, event.feed_url.clone(), occurred_at);
                            }
                            Ok(())
                        }
                        Err(err) => Err(err.into()),
                    }
                }
                TimelineProjectionInput::EntryChanged { event, .. } => {
                    match scope
                        .apply_entry_changed(&event.feed_url, &event.entry_id, occurred_at)
                        .await
                    {
                        Ok(timelines) => {
                            for timeline in timelines {
                                invalidations.mark(timeline, event.feed_url.clone(), occurred_at);
                            }
                            Ok(())
                        }
                        Err(err) => Err(err.into()),
                    }
                }
            };

            if let Err(err) = consumed {
                skip_permanent_error(processor, err, "input")?;
            }
        }

        Ok(invalidations.into_events())
    }
}

/// Transaction-scoped operations for projecting timeline membership.
pub struct TimelineProjectionScope<'a, Tx> {
    tx: &'a mut Tx,
}

impl<'a, Tx> TimelineProjectionScope<'a, Tx> {
    pub fn new(tx: &'a mut Tx) -> Self {
        Self { tx }
    }
}

impl<Tx> TimelineProjectionScope<'_, Tx>
where
    Tx: TimelineTx + Send,
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

    pub async fn apply_entry_discovered(
        &mut self,
        feed_url: &FeedUrl,
        entry_id: &EntryId,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<Vec<TimelineKey>> {
        self.tx
            .apply_entry_discovered(feed_url, entry_id, now)
            .await
    }

    pub async fn apply_entry_changed(
        &mut self,
        feed_url: &FeedUrl,
        entry_id: &EntryId,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<Vec<TimelineKey>> {
        self.tx.apply_entry_changed(feed_url, entry_id, now).await
    }

    pub async fn apply_feed_unsubscribed(
        &mut self,
        subscription: &SubscriptionKey,
    ) -> RegistryDbResult<Option<TimelineKey>> {
        self.tx.apply_feed_unsubscribed(subscription).await
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

    fn mark(&mut self, timeline: TimelineKey, feed_url: FeedUrl, changed_at: DateTime<Utc>) {
        if let Some(change) = self
            .changes
            .iter_mut()
            .find(|change| change.timeline == timeline)
        {
            change.changed_at = change.changed_at.max(changed_at);
            if !change.affected_feeds.contains(&feed_url) {
                change.affected_feeds.push(feed_url);
            }
            return;
        }
        self.changes.push(TimelineInvalidation {
            timeline,
            changed_at,
            affected_feeds: vec![feed_url],
        });
    }

    fn into_events(self) -> Vec<Event> {
        self.changes
            .into_iter()
            .map(|change| {
                TimelineChangedEvent::new(change.timeline, change.changed_at, change.affected_feeds)
                    .into()
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TimelineInvalidation {
    timeline: TimelineKey,
    changed_at: DateTime<Utc>,
    affected_feeds: Vec<FeedUrl>,
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::subscription::SubscriberId;

    #[test]
    fn timeline_invalidations_keep_changed_at_per_timeline() {
        let first_time = Utc.with_ymd_and_hms(2026, 6, 8, 12, 0, 0).unwrap();
        let second_time = Utc.with_ymd_and_hms(2026, 6, 8, 12, 1, 0).unwrap();
        let timeline_a = TimelineKey::default_for(SubscriberId::new("reader-a"));
        let timeline_b = TimelineKey::default_for(SubscriberId::new("reader-b"));
        let feed_a = FeedUrl::parse("https://example.com/a.xml").unwrap();
        let feed_b = FeedUrl::parse("https://example.com/b.xml").unwrap();

        let mut invalidations = TimelineInvalidations::empty();
        invalidations.mark(timeline_a.clone(), feed_a.clone(), first_time);
        invalidations.mark(timeline_b.clone(), feed_b.clone(), second_time);

        let events = invalidations.into_events();
        assert_eq!(events.len(), 2);
        let Event::TimelineChanged(event_a) = &events[0] else {
            panic!("expected timeline changed event");
        };
        let Event::TimelineChanged(event_b) = &events[1] else {
            panic!("expected timeline changed event");
        };
        assert_eq!(event_a.timeline, timeline_a);
        assert_eq!(event_a.changed_at, first_time);
        assert_eq!(event_a.affected_feeds, vec![feed_a]);
        assert_eq!(event_b.timeline, timeline_b);
        assert_eq!(event_b.changed_at, second_time);
        assert_eq!(event_b.affected_feeds, vec![feed_b]);
    }
}
