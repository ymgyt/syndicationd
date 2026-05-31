mod api_event;
mod sub_request;

pub use api_event::{ApiEventProj, ApiEventProjectionInput};
pub use sub_request::{SubRequestInput, SubRequestProj};

use crate::event::{Event, ProcessorError};

fn unexpected_event(expected: &'static str, event: &Event) -> ProcessorError {
    ProcessorError::UnexpectedEvent {
        expected,
        actual: event.kind(),
    }
}
