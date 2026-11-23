use chrono::{DateTime, Utc};
use synd_feed::{entry::EntryId, types::FeedUrl};
use tracing::info;

use crate::{
    db::{FeedRegistryDb, TimelineDb},
    event::{
        EntryChangedEvent, EntryDiscoveredEvent, Event, EventInput, EventType, FeedSubscribedEvent,
        FeedUnsubscribedEvent, InputBatch, Processor, ProcessorError, ProcessorId, ProcessorResult,
        Projector, RegistryEvent, TimelineChangedEvent, TimelineCreatedEvent,
    },
    subscription::SubscriberId,
};

/// Event input used to project timeline state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineProjInput {
    TimelineCreated(TimelineCreatedEvent),
    FeedSubscribed(FeedSubscribedEvent),
    FeedUnsubscribed(FeedUnsubscribedEvent),
    EntryDiscovered(EntryDiscoveredEvent),
    EntryChanged(EntryChangedEvent),
}

impl EventInput for TimelineProjInput {
    const INTERESTS: &'static [EventType] = &[
        TimelineCreatedEvent::TYPE,
        FeedSubscribedEvent::TYPE,
        FeedUnsubscribedEvent::TYPE,
        EntryDiscoveredEvent::TYPE,
        EntryChangedEvent::TYPE,
    ];

    fn from_event(event: Event, _occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::TimelineCreated(event) => Ok(Self::TimelineCreated(event)),
            Event::FeedSubscribed(event) => Ok(Self::FeedSubscribed(event)),
            Event::FeedUnsubscribed(event) => Ok(Self::FeedUnsubscribed(event)),
            Event::EntryDiscovered(event) => Ok(Self::EntryDiscovered(event)),
            Event::EntryChanged(event) => Ok(Self::EntryChanged(event)),
            event => Err(ProcessorError::unexpected_input(
                "timeline projection event",
                &event,
            )),
        }
    }
}

/// Updates timeline membership from timeline, subscription, and entry events.
#[derive(Debug, Clone)]
pub struct TimelineProj;

impl TimelineProj {
    pub fn new() -> Self {
        Self
    }

    async fn update_membership<Tx>(
        tx: &mut Tx,
        input: TimelineProjInput,
        invalidations: &mut TimelineInvalidations,
    ) -> ProcessorResult<()>
    where
        Tx: TimelineDb + Send,
    {
        match input {
            TimelineProjInput::TimelineCreated(event) => {
                let feeds = tx.catchup_created_timeline(event.timeline_id).await?;
                for feed_url in feeds {
                    invalidations.mark(event.subscriber_id.clone(), feed_url);
                }
            }
            TimelineProjInput::FeedSubscribed(event) => {
                let subscription = event.subscription;
                let catchup = tx
                    .catchup_subscribed_feed(&subscription.subscriber_id, &subscription.feed_url)
                    .await?;
                if catchup.inserted_items() > 0 {
                    invalidations.mark(catchup.subscriber_id().clone(), catchup.feed_url().clone());
                }
            }
            TimelineProjInput::FeedUnsubscribed(event) => {
                let subscription = event.subscription;
                if let Some(subscriber_id) = tx.apply_feed_unsubscribed(&subscription).await? {
                    invalidations.mark(subscriber_id, subscription.feed_url);
                }
            }
            TimelineProjInput::EntryDiscovered(event) => {
                Self::project_entry(tx, &event.feed_url, &event.entry_id, false, invalidations)
                    .await?;
            }
            TimelineProjInput::EntryChanged(event) => {
                Self::project_entry(tx, &event.feed_url, &event.entry_id, true, invalidations)
                    .await?;
            }
        }
        Ok(())
    }

    async fn project_entry<Tx>(
        tx: &mut Tx,
        feed_url: &FeedUrl,
        entry_id: &EntryId,
        content_changed: bool,
        invalidations: &mut TimelineInvalidations,
    ) -> ProcessorResult<()>
    where
        Tx: TimelineDb + Send,
    {
        let subscribers = tx
            .apply_entry_to_timelines(feed_url, entry_id, content_changed)
            .await?;
        for subscriber_id in subscribers {
            invalidations.mark(subscriber_id, feed_url.clone());
        }
        Ok(())
    }
}

impl Default for TimelineProj {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for TimelineProj {
    fn id(&self) -> ProcessorId {
        ProcessorId::TimelineProjection
    }

    type Input = TimelineProjInput;
}

impl<S> Projector<S> for TimelineProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: TimelineDb + Send,
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
            if let Err(err) = Self::update_membership(tx, input, &mut invalidations).await {
                err.skip_permanent(processor, "input")?;
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

    fn mark(&mut self, subscriber_id: SubscriberId, feed_url: FeedUrl) {
        if let Some(change) = self
            .changes
            .iter_mut()
            .find(|change| change.subscriber_id == subscriber_id)
        {
            if !change.affected_feeds.contains(&feed_url) {
                change.affected_feeds.push(feed_url);
            }
            return;
        }
        self.changes.push(TimelineInvalidation {
            subscriber_id,
            affected_feeds: vec![feed_url],
        });
    }

    fn into_events(self) -> Vec<Event> {
        self.changes
            .into_iter()
            .map(|change| {
                TimelineChangedEvent::new(change.subscriber_id, change.affected_feeds).into()
            })
            .collect()
    }

    fn log_changes(&self) {
        for change in &self.changes {
            info!(
                subscriber_id = change.subscriber_id.as_str(),
                affected_feeds = change.affected_feeds.len(),
                "timeline changed"
            );
        }
    }
}

/// A timeline membership change grouped by affected feeds.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TimelineInvalidation {
    subscriber_id: SubscriberId,
    affected_feeds: Vec<FeedUrl>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subscription::SubscriberId;

    #[test]
    fn timeline_invalidations_group_feeds_per_subscriber() {
        let subscriber_a = SubscriberId::new("reader-a");
        let subscriber_b = SubscriberId::new("reader-b");
        let feed_a = FeedUrl::parse("https://example.com/a.xml").unwrap();
        let feed_b = FeedUrl::parse("https://example.com/b.xml").unwrap();

        let mut invalidations = TimelineInvalidations::empty();
        invalidations.mark(subscriber_a.clone(), feed_a.clone());
        invalidations.mark(subscriber_b.clone(), feed_b.clone());

        let events = invalidations.into_events();
        assert_eq!(events.len(), 2);
        let Event::TimelineChanged(event_a) = &events[0] else {
            panic!("expected timeline changed event");
        };
        let Event::TimelineChanged(event_b) = &events[1] else {
            panic!("expected timeline changed event");
        };
        assert_eq!(event_a.subscriber_id, subscriber_a);
        assert_eq!(event_a.affected_feeds, vec![feed_a]);
        assert_eq!(event_b.subscriber_id, subscriber_b);
        assert_eq!(event_b.affected_feeds, vec![feed_b]);
    }
}
