//! API-facing event projection and stream delivery.

mod event;
mod projection;
mod publisher;

pub use event::{ApiEvent, ApiTimelineChanged};
pub use projection::{ApiEventProj, ApiEventProjectionInput};
pub use publisher::{ApiEventPublisher, ApiEventRecvError, ApiEventSubscriber};
