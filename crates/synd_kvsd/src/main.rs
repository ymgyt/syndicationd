use std::env;

use synd_kvsd::{
    args::{self, ObservabilityOptions},
    config,
};
use synd_o11y::{tracing_subscriber::initializer::TracingInitializer, OpenTelemetryGuard};
use synd_stdx::color::{is_color_supported, ColorSupport};

fn init_tracing(options: &ObservabilityOptions) -> Option<OpenTelemetryGuard> {
    let ObservabilityOptions {
        show_code_location,
        show_target,
        otlp_endpoint,
        trace_sampler_ratio,
    } = options;

    TracingInitializer::default()
        .app_name(config::app::NAME)
        .app_version(config::app::VERSION)
        .otlp_endpoint(otlp_endpoint.clone())
        .trace_sampler_ratio(*trace_sampler_ratio)
        .enable_ansi(is_color_supported() == ColorSupport::Supported)
        .show_code_location(*show_code_location)
        .show_target(*show_target)
        .init()
}

#[tokio::main]
async fn main() {
    let args = match args::try_parse(env::args_os()) {
        Ok(args) => args,
        Err(err) => err.exit(),
    };
    let _guard = init_tracing(&args.o11y);
}
