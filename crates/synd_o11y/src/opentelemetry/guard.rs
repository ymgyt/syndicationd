/// `OpenTelemetry` terminination process handler
pub struct OpenTelemetryGuard;

impl Drop for OpenTelemetryGuard {
    fn drop(&mut self) {
        opentelemetry::global::shutdown_tracer_provider();
        opentelemetry::global::shutdown_meter_provider();
        opentelemetry::global::shutdown_logger_provider();
    }
}
