mod resource;
pub use resource::resource;

mod propagation;
pub use propagation::{extension, http, init_propagation};

pub use opentelemetry::KeyValue;
