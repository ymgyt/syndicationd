use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tracing::info;

use crate::{
    db::{FeedRegistryDb, TimelineStore},
    event::{
        EntryChangedEvent, EntryDiscoveredEvent, Event, EventInput, EventType, FeedSubscribedEvent,
        FeedUnsubscribedEvent, InputBatch, Processor, ProcessorError, ProcessorId, ProcessorResult,
        Projector, RegistryEvent, TimelineChangedEvent, skip_permanent_error,
    },
    timeline::TimelineKey,
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

impl EventInput for TimelineProjectionInput {
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

impl<S> Projector<S> for TimelineProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: TimelineStore + Send,
{
    async fn project(
        &mut self,
        tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        <Self as Projector<S>>::project_batch(self, tx, InputBatch::new(vec![input])).await
    }

    async fn project_batch(
        &mut self,
        tx: &mut S::Tx<'_>,
        batch: InputBatch<Self::Input>,
    ) -> ProcessorResult<Vec<Event>> {
        let processor = self.id();
        let mut invalidations = TimelineInvalidations::empty();

        for input in batch.into_inputs() {
            let occurred_at = input.occurred_at();
            let consumed = match input {
                TimelineProjectionInput::FeedSubscribed { event, .. } => {
                    let subscription = event.subscription;
                    let timeline = TimelineKey::default_for(subscription.subscriber_id);
                    match tx
                        .catchup_subscribed_feed(&timeline, &subscription.feed_url, occurred_at)
                        .await
                    {
                        Ok(catchup) => {
                            if catchup.inserted_items() > 0 {
                                invalidations
                                    .mark(catchup.timeline().clone(), catchup.feed_url().clone());
                            }
                            Ok(())
                        }
                        Err(err) => Err(err.into()),
                    }
                }
                TimelineProjectionInput::FeedUnsubscribed { event, .. } => {
                    let subscription = event.subscription;
                    match tx.apply_feed_unsubscribed(&subscription).await {
                        Ok(timeline) => {
                            if let Some(timeline) = timeline {
                                invalidations.mark(timeline, subscription.feed_url);
                            }
                            Ok(())
                        }
                        Err(err) => Err(err.into()),
                    }
                }
                TimelineProjectionInput::EntryDiscovered { event, .. } => {
                    match tx
                        .apply_entry_to_timelines(&event.feed_url, &event.entry_id, occurred_at)
                        .await
                    {
                        Ok(timelines) => {
                            for timeline in timelines {
                                invalidations.mark(timeline, event.feed_url.clone());
                            }
                            Ok(())
                        }
                        Err(err) => Err(err.into()),
                    }
                }
                TimelineProjectionInput::EntryChanged { event, .. } => {
                    match tx
                        .apply_entry_to_timelines(&event.feed_url, &event.entry_id, occurred_at)
                        .await
                    {
                        Ok(timelines) => {
                            for timeline in timelines {
                                invalidations.mark(timeline, event.feed_url.clone());
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

        invalidations.log_changes();
        Ok(invalidations.into_events())
    }
}

/// Coalesced timeline changes produced by one projection batch.
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

    fn into_events(self) -> Vec<Event> {
        self.changes
            .into_iter()
            .map(|change| TimelineChangedEvent::new(change.timeline, change.affected_feeds).into())
            .collect()
    }

    fn log_changes(&self) {
        for change in &self.changes {
            info!(
                subscriber_id = change.timeline.subscriber_id.as_str(),
                timeline = change.timeline.kind.as_str(),
                affected_feeds = change.affected_feeds.len(),
                "timeline changed"
            );
        }
    }
}

/// A timeline membership change grouped by affected feeds.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TimelineInvalidation {
    timeline: TimelineKey,
    affected_feeds: Vec<FeedUrl>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subscription::SubscriberId;

    #[test]
    fn timeline_invalidations_group_feeds_per_timeline() {
        let timeline_a = TimelineKey::default_for(SubscriberId::new("reader-a"));
        let timeline_b = TimelineKey::default_for(SubscriberId::new("reader-b"));
        let feed_a = FeedUrl::parse("https://example.com/a.xml").unwrap();
        let feed_b = FeedUrl::parse("https://example.com/b.xml").unwrap();

        let mut invalidations = TimelineInvalidations::empty();
        invalidations.mark(timeline_a.clone(), feed_a.clone());
        invalidations.mark(timeline_b.clone(), feed_b.clone());

        let events = invalidations.into_events();
        assert_eq!(events.len(), 2);
        let Event::TimelineChanged(event_a) = &events[0] else {
            panic!("expected timeline changed event");
        };
        let Event::TimelineChanged(event_b) = &events[1] else {
            panic!("expected timeline changed event");
        };
        assert_eq!(event_a.timeline, timeline_a);
        assert_eq!(event_a.affected_feeds, vec![feed_a]);
        assert_eq!(event_b.timeline, timeline_b);
        assert_eq!(event_b.affected_feeds, vec![feed_b]);
    }
}
