use crate::operation::Operation;

use super::Application;

impl Application {
    pub(super) fn dispatch(&mut self, operation: Operation) {
        for event in self.drivers.dispatch(operation) {
            self.apply_event(event);
        }
    }

    pub(super) fn dispatch_blk<I>(&mut self, operations: I)
    where
        I: IntoIterator<Item = Operation>,
    {
        for operation in operations {
            self.dispatch(operation);
        }
    }
}
