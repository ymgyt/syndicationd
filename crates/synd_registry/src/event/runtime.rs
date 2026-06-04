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

    pub async fn submit<I, E>(&self, events: I) -> EventSubmitterResult<RecordedEvents>
    where
        I: IntoIterator<Item = E>,
        E: Into<Event>,
    {
        let events = events.into_iter();
        let mut kinds = Vec::with_capacity(events.size_hint().0);
        let mut tx = self.db.begin().await?;
        for event in events {
            let event = event.into();
            kinds.push(event.kind());
            tx.append_event(event).await?;
        }
        tx.commit().await?;
        let recorded = RecordedEvents::new(kinds);
        self.wake_publisher.publish(recorded.clone());
        Ok(recorded)
    }
}
