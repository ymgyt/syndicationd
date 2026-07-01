use crate::crawl::scheduler::{
    dispatch::{DispatchBatch, DispatchContext},
    input::SchedInput,
};

/// Policy interface that decides crawl dispatch order from submitted inputs.
pub(crate) trait Scheduler {
    fn submit(&mut self, input: SchedInput);
    fn dispatch(&mut self, cx: DispatchContext) -> DispatchBatch;
}
