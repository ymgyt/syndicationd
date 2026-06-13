use std::sync::Arc;

use synd_support::time::{Clock, SystemClock};
use thiserror::Error;

use crate::{
    db::{CommitTx, FeedRegistryDb},
    error::RegistryDbError,
    event::{Event, EventRecorder, EventWakePublisher, JournalAppendTx, RecordedEvents},
};

pub type EventSubmitterResult<T> = Result<T, EventSubmitterError>;

#[derive(Debug, Error)]
pub enum EventSubmitterError {
    #[error(transparent)]
    Db(#[from] RegistryDbError),
}

/// Records submitted events and wakes registry event workers.
#[derive(Clone)]
pub struct EventSubmitter<S> {
    db: S,
    wake_publisher: EventWakePublisher,
    clock: Arc<dyn Clock>,
}

impl<S> EventSubmitter<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: JournalAppendTx,
{
    pub fn new(db: S, wake_publisher: EventWakePublisher) -> Self {
        Self::with_clock(db, wake_publisher, Arc::new(SystemClock))
    }

    pub fn with_clock(db: S, wake_publisher: EventWakePublisher, clock: Arc<dyn Clock>) -> Self {
        Self {
            db,
            wake_publisher,
            clock,
        }
    }

    pub async fn submit<I, E>(&self, events: I) -> EventSubmitterResult<RecordedEvents>
    where
        I: IntoIterator<Item = E>,
        E: Into<Event>,
    {
        let events = events.into_iter();
        let mut tx = self.db.begin().await?;
        let mut recorded_events = RecordedEvents::with_capacity(events.size_hint().0);
        {
            let mut event_recorder =
                EventRecorder::new(&mut tx, &mut recorded_events, self.clock.as_ref());
            event_recorder.record_all(events).await?;
        }
        tx.commit().await?;
        self.wake_publisher.publish(recorded_events.clone());
        Ok(recorded_events)
    }
}
