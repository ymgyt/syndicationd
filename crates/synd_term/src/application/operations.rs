use crate::operation::Operation;

use super::Application;

impl Application {
    pub(super) fn perform_operation(&mut self, operation: Operation) {
        for event in self.drivers.perform_operation(operation) {
            self.apply_event(event);
        }
    }

    pub(super) fn perform_operations<I>(&mut self, operations: I)
    where
        I: IntoIterator<Item = Operation>,
    {
        for operation in operations {
            self.perform_operation(operation);
        }
    }
}
