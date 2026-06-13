use chrono::{DateTime, Utc};
use synd_support::time::Clock;

use crate::{
    error::RegistryDbResult,
    event::{Event, JournalAppendTx, RecordedEvents},
};

/// Records journal events and the wake summary as one operation.
pub struct EventRecorder<'a, Tx, C: ?Sized> {
    tx: &'a mut Tx,
    recorded: &'a mut RecordedEvents,
    clock: &'a C,
}

impl<'a, Tx, C: ?Sized> EventRecorder<'a, Tx, C> {
    pub fn new(tx: &'a mut Tx, recorded: &'a mut RecordedEvents, clock: &'a C) -> Self {
        Self {
            tx,
            recorded,
            clock,
        }
    }
}

impl<Tx, C> EventRecorder<'_, Tx, C>
where
    Tx: JournalAppendTx + Send,
    C: Clock + ?Sized,
{
    pub async fn record<E>(&mut self, event: E) -> RegistryDbResult<JournalEventMeta>
    where
        E: Into<Event>,
    {
        let occurred_at = self.clock.now();
        #[expect(
            clippy::disallowed_methods,
            reason = "EventRecorder is the recording boundary for journal appends"
        )]
        let event_type = self.tx.append_event(event.into(), occurred_at).await?;
        self.recorded.push(event_type);
        Ok(JournalEventMeta { occurred_at })
    }

    pub async fn record_all<I, E>(&mut self, events: I) -> RegistryDbResult<()>
    where
        I: IntoIterator<Item = E>,
        E: Into<Event>,
    {
        for event in events {
            self.record(event).await?;
        }
        Ok(())
    }
}

/// Metadata assigned to an event by the journal recording boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalEventMeta {
    pub occurred_at: DateTime<Utc>,
}
