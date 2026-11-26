use chrono::{DateTime, Utc};

use crate::{error::RegistryDbResult, event::RecordedEvents};

/// Result of a command transaction after DB state and journal have been committed.
pub(crate) struct HandledCommand<O> {
    pub output: O,
    pub recorded_events: RecordedEvents,
}

/// Runs one registry command from state load through transaction commit.
pub(crate) trait CommandHandler<C> {
    type Output;
    type Error;

    async fn handle(&self, command: C) -> Result<HandledCommand<Self::Output>, Self::Error>;
}

/// Applies newly decided domain events to transactional current state.
pub(crate) trait StateApplier<Tx> {
    type Event;

    async fn apply(
        &self,
        tx: &mut Tx,
        event: &Self::Event,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<()>;
}
