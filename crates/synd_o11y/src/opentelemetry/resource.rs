use opentelemetry::KeyValue;
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::resource::{
    DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_NAMESPACE, SERVICE_VERSION,
};
use std::borrow::Cow;

pub fn resource(
    service_name: impl Into<Cow<'static, str>>,
    service_version: impl Into<Cow<'static, str>>,
    deployment_environment: impl Into<Cow<'static, str>>,
) -> Resource {
    Resource::new(
        [
            (SERVICE_NAME, service_name.into()),
            (SERVICE_VERSION, service_version.into()),
            (SERVICE_NAMESPACE, "syndicationd".into()),
            (DEPLOYMENT_ENVIRONMENT, deployment_environment.into()),
        ]
        .into_iter()
        .map(|(key, value)| KeyValue::new(key, value)),
    )
}
