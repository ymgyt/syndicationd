use crate::crawl::{
    dispatch::{DispatchBatch, DispatchContext},
    scheduler::input::SchedInput,
};

/// Policy interface that decides crawl dispatch order from submitted inputs.
pub(crate) trait Scheduler: Send {
    fn submit(&mut self, input: SchedInput);
    fn dispatch(&mut self, cx: DispatchContext) -> DispatchBatch;
}
