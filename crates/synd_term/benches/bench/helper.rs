use std::time::Duration;

use ratatui::backend::TestBackend;
use synd_term::{
    application::{Application, Cache, Config},
    client::Client,
    config::Categories,
    terminal::Terminal,
    ui::theme::Theme,
};
use url::Url;

pub fn init_app() -> Application {
    let terminal = {
        let backend = TestBackend::new(120, 40);
        let terminal = ratatui::Terminal::new(backend).unwrap();
        Terminal::with(terminal)
    };

    let client = {
        Client::new(
            Url::parse("http://dummy.ymgyt.io").unwrap(),
            Duration::from_secs(10),
        )
        .unwrap()
    };

    let config = { Config::default().with_idle_timer_interval(Duration::from_micros(1)) };

    let cache = { Cache::new(tempfile::TempDir::new().unwrap().into_path()) };

    Application::builder()
        .terminal(terminal)
        .client(client)
        .categories(Categories::default_toml())
        .config(config)
        .cache(cache)
        .theme(Theme::default())
        .build()
}
