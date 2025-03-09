use opentelemetry::KeyValue;
use opentelemetry_sdk::resource::EnvResourceDetector;
use opentelemetry_semantic_conventions::{
    SCHEMA_URL,
    resource::{SERVICE_NAME, SERVICE_NAMESPACE, SERVICE_VERSION},
};
use std::{borrow::Cow, time::Duration};

pub use opentelemetry_sdk::Resource;

/// Return the [`Resource`] of opentelemetry.
/// Check and merge the environment variables specified in the specification.
pub fn resource(
    service_name: impl Into<Cow<'static, str>>,
    service_version: impl Into<Cow<'static, str>>,
) -> Resource {
    Resource::from_schema_url(
        [
            (SERVICE_NAME, service_name.into()),
            (SERVICE_VERSION, service_version.into()),
            (SERVICE_NAMESPACE, "syndicationd".into()),
        ]
        .into_iter()
        .map(|(key, value)| KeyValue::new(key, value)),
        SCHEMA_URL,
    )
    .merge(&Resource::from_detectors(
        Duration::from_millis(200),
        // Detect "OTEL_RESOURCE_ATTRIBUTES" environment variables
        vec![Box::new(EnvResourceDetector::new())],
    ))
}
