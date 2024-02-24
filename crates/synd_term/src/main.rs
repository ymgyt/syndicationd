use std::path::PathBuf;

use crossterm::event::EventStream;
use synd_term::{
    application::Application,
    auth,
    cli::{self, Args},
    client::Client,
    config,
    terminal::Terminal,
    ui::theme::Theme,
};
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

fn init_tracing(log_path: Option<PathBuf>) -> anyhow::Result<Option<WorkerGuard>> {
    use synd_o11y::opentelemetry::init_propagation;
    use tracing_subscriber::{
        filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt as _, Registry,
    };

    let (writer, guard) = if let Some(log_path) = log_path {
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let log = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(log_path)?;
        let (non_blocking, guard) = tracing_appender::non_blocking(log);
        (BoxMakeWriter::new(non_blocking), Some(guard))
    } else {
        (BoxMakeWriter::new(std::io::stdout), None)
    };

    Registry::default()
        .with(
            fmt::Layer::new()
                .with_ansi(true)
                .with_timer(fmt::time::UtcTime::rfc_3339())
                .with_file(false)
                .with_line_number(false)
                .with_target(true)
                .with_writer(writer),
        )
        .with(
            EnvFilter::try_from_env(config::env::LOG_DIRECTIVE)
                .or_else(|_| EnvFilter::try_new("info"))
                .unwrap(),
        )
        .try_init()?;

    // Set text map progator globally
    init_propagation();

    Ok(guard)
}

#[tokio::main]
async fn main() {
    let Args {
        endpoint,
        log,
        command,
        palette,
        timeout,
    } = cli::parse();

    let log = if command.is_some() { None } else { Some(log) };
    let _guard = init_tracing(log).unwrap();

    #[allow(clippy::single_match)]
    match command {
        Some(cli::Command::Clear(clear)) => clear.run(),
        None => {}
    }

    let mut app = {
        let terminal = Terminal::new().expect("Failed to construct terminal");
        let client = Client::new(endpoint, timeout).expect("Failed to construct client");
        Application::new(terminal, client).with_theme(Theme::with_palette(&palette.into()))
    };

    if let Some(auth) = auth::credential_from_cache() {
        info!("Use authentication cache");
        app.set_credential(auth);
    }

    info!("Running...");

    if let Err(err) = app.run(&mut EventStream::new()).await {
        error!("{err}");
        std::process::exit(1);
    }
}
