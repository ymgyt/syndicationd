use crate::{
    OpenTelemetryGuard,
    opentelemetry::init_propagation,
    opentelemetry_layer,
    tracing_subscriber::{audit, otel_metrics::metrics_event_filter},
};

const LOG_DIRECTIVE_ENV: &str = "SYND_LOG";
const DEFAULT_LOG_DIRECTIVE: &str = "info";

pub struct TracingInitializer {
    app_name: Option<&'static str>,
    app_version: Option<&'static str>,
    otlp_endpoint: Option<String>,
    trace_sampler_ratio: f64,
    enable_ansi: bool,
    show_code_location: bool,
    show_target: bool,
    log_directive_env: Option<&'static str>,
}

impl Default for TracingInitializer {
    fn default() -> Self {
        Self {
            app_name: None,
            app_version: None,
            otlp_endpoint: None,
            trace_sampler_ratio: 1.,
            enable_ansi: true,
            show_code_location: false,
            show_target: true,
            log_directive_env: None,
        }
    }
}

impl TracingInitializer {
    /// Initialize tracing Subscriber with layers
    pub fn init(self) -> Option<OpenTelemetryGuard> {
        use tracing_subscriber::{
            Layer as _, Registry, filter::EnvFilter, fmt, layer::SubscriberExt,
            util::SubscriberInitExt as _,
        };

        let Self {
            app_name: Some(app_name),
            app_version: Some(app_version),
            otlp_endpoint,
            trace_sampler_ratio,
            enable_ansi,
            show_code_location,
            show_target,
            log_directive_env,
        } = self
        else {
            panic!()
        };

        let (otel_layer, otel_guard) =
            match otlp_endpoint
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|endpoint| {
                    opentelemetry_layer(endpoint, app_name, app_version, trace_sampler_ratio)
                }) {
                Some((otel_layer, otel_guard)) => (Some(otel_layer), Some(otel_guard)),
                _ => (None, None),
            };

        Registry::default()
            .with(
                fmt::Layer::new()
                    .with_ansi(enable_ansi)
                    .with_timer(fmt::time::ChronoLocal::rfc_3339())
                    .with_file(show_code_location)
                    .with_line_number(show_code_location)
                    .with_target(show_target)
                    .with_filter(metrics_event_filter())
                    .and_then(otel_layer)
                    .with_filter(
                        EnvFilter::try_from_env(log_directive_env.unwrap_or(LOG_DIRECTIVE_ENV))
                            .or_else(|_| EnvFilter::try_new(DEFAULT_LOG_DIRECTIVE))
                            .unwrap()
                            .add_directive(audit::Audit::directive()),
                    ),
            )
            .with(audit::layer())
            .init();

        // Set text map propagator globally
        init_propagation();

        otel_guard
    }
}

impl TracingInitializer {
    #[must_use]
    pub fn app_name(self, app_name: &'static str) -> Self {
        Self {
            app_name: Some(app_name),
            ..self
        }
    }

    #[must_use]
    pub fn app_version(self, app_version: &'static str) -> Self {
        Self {
            app_version: Some(app_version),
            ..self
        }
    }

    #[must_use]
    pub fn otlp_endpoint(self, otlp_endpoint: Option<String>) -> Self {
        Self {
            otlp_endpoint,
            ..self
        }
    }

    #[must_use]
    pub fn trace_sampler_ratio(self, trace_sampler_ratio: f64) -> Self {
        Self {
            trace_sampler_ratio,
            ..self
        }
    }

    #[must_use]
    pub fn enable_ansi(self, enable_ansi: bool) -> Self {
        Self {
            enable_ansi,
            ..self
        }
    }

    #[must_use]
    pub fn show_code_location(self, show_code_location: bool) -> Self {
        Self {
            show_code_location,
            ..self
        }
    }

    #[must_use]
    pub fn show_target(self, show_target: bool) -> Self {
        Self {
            show_target,
            ..self
        }
    }

    #[must_use]
    pub fn log_directive_env(self, log_directive_env: &'static str) -> Self {
        Self {
            log_directive_env: Some(log_directive_env),
            ..self
        }
    }
}
