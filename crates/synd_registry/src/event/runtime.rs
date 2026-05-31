use thiserror::Error;

use crate::{
    db::{CommitTx, FeedRegistryDb},
    error::RegistryDbError,
    event::{Event, EventWakePublisher, JournalTx, RecordedEvents},
};

pub type EventSubmitterResult<T> = Result<T, EventSubmitterError>;

#[derive(Debug, Error)]
pub enum EventSubmitterError {
    #[error(transparent)]
    Db(#[from] RegistryDbError),
}

/// Records submitted events and wakes registry event workers.
#[derive(Debug, Clone)]
pub struct EventSubmitter<S> {
    db: S,
    wake_publisher: EventWakePublisher,
}

impl<S> EventSubmitter<S>
where
    S: FeedRegistryDb,
{
    pub fn new(db: S, wake_publisher: EventWakePublisher) -> Self {
        Self { db, wake_publisher }
    }

    pub async fn submit(&self, events: Vec<Event>) -> EventSubmitterResult<RecordedEvents> {
        let mut kinds = Vec::with_capacity(events.len());
        let mut tx = self.db.begin().await?;
        for event in events {
            kinds.push(event.kind());
            tx.append_event(event).await?;
        }
        tx.commit().await?;
        let recorded = RecordedEvents::new(kinds);
        self.wake_publisher.publish(recorded.clone());
        Ok(recorded)
    }
}
