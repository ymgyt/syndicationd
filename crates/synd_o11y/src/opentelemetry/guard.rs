use opentelemetry_sdk::{
    logs::SdkLoggerProvider, metrics::SdkMeterProvider, trace::SdkTracerProvider,
};
use tracing::warn;

/// `OpenTelemetry` terminination process handler
pub struct OpenTelemetryGuard {
    pub(crate) tracer_provider: SdkTracerProvider,
    pub(crate) meter_provider: SdkMeterProvider,
    pub(crate) logger_provider: SdkLoggerProvider,
}

impl Drop for OpenTelemetryGuard {
    fn drop(&mut self) {
        // https://github.com/open-telemetry/opentelemetry-rust/blob/main/docs/migration_0.28.md#tracing-shutdown-changes
        if let Err(err) = self.tracer_provider.shutdown() {
            warn!("{err}");
        }
        if let Err(err) = self.meter_provider.shutdown() {
            warn!("{err}");
        }
        if let Err(err) = self.logger_provider.shutdown() {
            warn!("{err}");
        }
    }
}
