use std::time::Duration;

use synd_client::{Client, ClientOptions};
use synd_term::{
    application::{Application, Cache, ClientFeedApi, Config},
    config::Categories,
    integration::TestApplication,
    interact::mock::MockInteractor,
    ui::theme::Theme,
};
use url::Url;

pub fn init_app() -> TestApplication {
    let client = {
        Client::new(
            Url::parse("http://dummy.ymgyt.io").unwrap(),
            ClientOptions::new(Duration::from_secs(10), "synd-term-bench"),
        )
        .unwrap()
    };

    let cache = { Cache::new(tempfile::TempDir::new().unwrap().keep()) };

    Application::builder()
        .feed_api(ClientFeedApi::new(client))
        .categories(Categories::default_toml())
        .config(Config::default())
        .cache(cache)
        .theme(Theme::default())
        .interactor(Box::new(MockInteractor::new()))
        .build()
        .with_test_terminal(120, 40)
}
