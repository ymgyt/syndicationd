use thiserror::Error;

use crate::event::{Event, EventJournal, EventJournalError, EventWakePublisher, RecordedEvents};

pub type EventSubmitterResult<T> = Result<T, EventSubmitterError>;

#[derive(Debug, Error)]
pub enum EventSubmitterError {
    #[error(transparent)]
    Journal(#[from] EventJournalError),
}

/// Records submitted events and wakes registry event workers.
#[derive(Debug, Clone)]
pub struct EventSubmitter<J> {
    journal: J,
    wake_publisher: EventWakePublisher,
}

impl<J> EventSubmitter<J>
where
    J: EventJournal,
{
    pub fn new(journal: J, wake_publisher: EventWakePublisher) -> Self {
        Self {
            journal,
            wake_publisher,
        }
    }

    pub async fn submit(&self, events: Vec<Event>) -> EventSubmitterResult<RecordedEvents> {
        let mut kinds = Vec::with_capacity(events.len());
        for event in events {
            kinds.push(event.kind());
            self.journal.append(event).await?;
        }
        let recorded = RecordedEvents::new(kinds);
        self.wake_publisher.publish(recorded.clone());
        Ok(recorded)
    }
}
