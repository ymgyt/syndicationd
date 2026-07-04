use chrono::{DateTime, Utc};

use crate::crawl::{
    dispatch::{DispatchContext, DispatchQueuePushError, DispatchQueueWriter},
    scheduler::{input::SchedInput, policy::Scheduler},
};

pub(crate) struct SchedDriver {
    scheduler: Box<dyn Scheduler>,
    dispatch_queue: DispatchQueueWriter,
}

impl SchedDriver {
    pub(crate) fn new(scheduler: Box<dyn Scheduler>, dispatch_queue: DispatchQueueWriter) -> Self {
        Self {
            scheduler,
            dispatch_queue,
        }
    }

    pub(crate) fn submit(&mut self, input: SchedInput) {
        self.scheduler.submit(input);
    }

    pub(crate) fn submit_batch<I>(&mut self, inputs: I)
    where
        I: IntoIterator<Item = SchedInput>,
    {
        for input in inputs {
            self.submit(input);
        }
    }

    pub(crate) fn dispatch(&mut self, now: DateTime<Utc>) -> Result<usize, DispatchQueuePushError> {
        let cx = DispatchContext::new(
            now,
            self.dispatch_queue.len(),
            self.dispatch_queue.remaining_capacity(),
        );
        let batch = self.scheduler.dispatch(cx);
        let dispatched = batch.len();

        for entry in batch.into_entries() {
            self.dispatch_queue.push(entry)?;
        }

        Ok(dispatched)
    }
}
