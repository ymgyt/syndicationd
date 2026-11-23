use std::sync::Arc;

use synd_support::time::Clock;

use crate::{
    CreateTimelineCommand, CreateTimelineOutput, FeedRegistryError, TimelineId,
    db::{CommitTx, FeedRegistryDb, InsertTimelineOutcome, TimelineDb},
    event::{EventRecorder, RecordedEvents, TimelineCreatedEvent},
    handler::{CommandHandler, HandledCommand},
};

#[derive(Clone)]
pub(crate) struct TimelineHandler<S> {
    db: S,
    clock: Arc<dyn Clock>,
}

impl<S> TimelineHandler<S> {
    pub(crate) fn new(db: S, clock: Arc<dyn Clock>) -> Self {
        Self { db, clock }
    }
}

impl<S> CommandHandler<CreateTimelineCommand> for TimelineHandler<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: TimelineDb,
{
    type Output = CreateTimelineOutput;
    type Error = FeedRegistryError;

    async fn handle(
        &self,
        command: CreateTimelineCommand,
    ) -> Result<HandledCommand<Self::Output>, Self::Error> {
        let timeline_id = TimelineId::generate();
        let event = TimelineCreatedEvent {
            timeline_id,
            subscriber_id: command.subscriber_id,
            name: command.name,
        };
        let mut tx = self.db.begin().await?;
        match tx.insert_timeline(&event).await? {
            InsertTimelineOutcome::Inserted => {}
            InsertTimelineOutcome::NameAlreadyExists => {
                return Err(FeedRegistryError::TimelineNameAlreadyExists { name: event.name });
            }
        }
        let mut recorded_events = RecordedEvents::with_capacity(1);
        EventRecorder::new(&mut tx, &mut recorded_events, self.clock.as_ref())
            .record(event)
            .await?;
        tx.commit().await?;
        Ok(HandledCommand {
            output: CreateTimelineOutput { timeline_id },
            recorded_events,
        })
    }
}
