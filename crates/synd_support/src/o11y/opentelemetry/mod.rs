mod resource;
pub use resource::{Resource, resource};

mod propagation;
pub use propagation::{http, init_propagation};

pub use opentelemetry::KeyValue;

mod guard;
pub use guard::OpenTelemetryGuard;

pub mod extension {
    pub use opentelemetry::baggage::BaggageExt as _;
    pub use tracing_opentelemetry::OpenTelemetrySpanExt as _;
}
