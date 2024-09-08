use std::env;

use synd_kvsd::{
    boot::Boot,
    cli::{self, ObservabilityOptions},
    config::{self, ConfigResolver},
};
use synd_o11y::{tracing_subscriber::initializer::TracingInitializer, OpenTelemetryGuard};
use synd_stdx::io::color::{is_color_supported, ColorSupport};

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
        .log_directive_env(config::env::LOG_DIRECTIVE)
        .init()
}

#[tokio::main]
async fn main() {
    let args = match cli::try_parse(env::args_os()) {
        Ok(args) => args,
        Err(err) => err.exit(),
    };
    let _guard = init_tracing(&args.o11y);

    // 1. Resolve Config
    let config = match ConfigResolver::from_args(args.kvsd).resolve() {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("{err}");
            std::process::exit(1);
        }
    };

    Boot::new(config.root_dir()).boot().unwrap();

    // 7. Spawn Kvsd
    // 8. Run Server
}
