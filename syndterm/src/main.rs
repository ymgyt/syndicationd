use crossterm::event::EventStream;
use syndterm::{
    application::Application,
    args::{self, Args},
    auth,
    client::Client,
    terminal::Terminal,
};
use tracing::error;

fn init_tracing() {
    use tracing_subscriber::{
        filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt as _, Registry,
    };

    // TODO: write to file

    Registry::default()
        .with(
            fmt::Layer::new()
                .with_ansi(false)
                .with_timer(fmt::time::UtcTime::rfc_3339())
                .with_file(false)
                .with_line_number(false)
                .with_target(true),
        )
        .with(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("info"))
                .unwrap(),
        )
        .init();
}

#[tokio::main]
async fn main() {
    init_tracing();

    let Args { endpoint, command } = args::parse();

    if let Some(command) = command {
        match command {
            args::Command::Login(cmd) => auth::login(cmd).await,
            args::Command::Logout => unimplemented!(),
        }
    }

    let auth = {
        match auth::authenticate_from_cache() {
            Some(auth) => auth,
            None => {
                eprintln!("Please login (`synd login oauth`)");
                std::process::exit(1);
            }
        }
    };

    let app = {
        let terminal =
            Terminal::from_stdout(std::io::stdout()).expect("Failed to construct terminal");
        let client = Client::new(endpoint, auth).expect("Failed to construct client");
        Application::new(terminal, client)
    };

    if let Err(err) = app.run(&mut EventStream::new()).await {
        error!("{err}");
        std::process::exit(1);
    }
}
