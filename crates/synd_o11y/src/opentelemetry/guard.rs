use opentelemetry_sdk::logs::LoggerProvider;

/// `OpenTelemetry` terminination process handler
pub struct OpenTelemetryGuard {
    pub(crate) logger_provider: LoggerProvider,
}

impl Drop for OpenTelemetryGuard {
    fn drop(&mut self) {
        opentelemetry::global::shutdown_tracer_provider();
        // global provider for logs is removed in v0.23.0
        // https://github.com/open-telemetry/opentelemetry-rust/blob/main/opentelemetry/CHANGELOG.md#removed
        self.logger_provider.shutdown().ok();

        // global::shutdown_meter_provider is removed in v0.22.0
        // https://github.com/open-telemetry/opentelemetry-rust/blob/main/opentelemetry/CHANGELOG.md#removed
    }
}
