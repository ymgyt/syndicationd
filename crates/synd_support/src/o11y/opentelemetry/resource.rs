use opentelemetry::KeyValue;
use opentelemetry_sdk::resource::EnvResourceDetector;
use opentelemetry_semantic_conventions::{
    SCHEMA_URL,
    resource::{SERVICE_NAME, SERVICE_NAMESPACE, SERVICE_VERSION},
};
use std::borrow::Cow;

pub use opentelemetry_sdk::Resource;

/// Return the [`Resource`] of opentelemetry.
/// Check and merge the environment variables specified in the specification.
pub fn resource(
    service_name: impl Into<Cow<'static, str>>,
    service_version: impl Into<Cow<'static, str>>,
) -> Resource {
    Resource::builder()
        .with_schema_url(
            [
                (SERVICE_NAME, service_name.into()),
                (SERVICE_VERSION, service_version.into()),
                (SERVICE_NAMESPACE, "syndicationd".into()),
            ]
            .into_iter()
            .map(|(key, value)| KeyValue::new(key, value)),
            SCHEMA_URL,
        )
        .with_detectors(&[Box::new(EnvResourceDetector::new())])
        .build()
}
