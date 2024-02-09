use std::future::pending;

use kvsd::client::Api;
use ratatui::backend::TestBackend;
use synd_api::{client::github::GithubClient, serve::auth::Authenticator};
use synd_term::terminal::Terminal;
use tokio::net::{TcpListener, TcpStream};

pub fn new_test_terminal() -> Terminal {
    let backend = TestBackend::new(80, 20);
    let terminal = ratatui::Terminal::new(backend).unwrap();
    Terminal::with(terminal)
}

// Dependency
// * serve
//   * tcp listener
//   * dependency
//     * authenticator
//       * github client
//         * github endpoint
//     * usecase runtime
//       * make usecase
//         * datastore
//         * fetch cached feed
//       * authorizer
#[allow(unused)]
pub fn serve_api(mock_port: u16) -> anyhow::Result<()> {
    let github_endpoint: &'static str =
        format!("http://localhost:{mock_port}/github/graphql").leak();
    let github_client = GithubClient::new()?.with_endpoint(github_endpoint);
    let authenticator = Authenticator::new()?.with_client(github_client);

    Ok(())
}

pub async fn run_kvsd() -> anyhow::Result<kvsd::client::tcp::Client<TcpStream>> {
    let root_dir = temp_dir();
    let mut config = kvsd::config::Config::default();

    // Setup user credential.
    config.kvsd.users = vec![kvsd::core::UserEntry {
        username: "test".into(),
        password: "test".into(),
    }];
    config.server.set_disable_tls(&mut Some(true));

    // Test Server listen addr
    let addr = ("localhost", 47379);

    let mut initializer = kvsd::config::Initializer::from_config(config);

    initializer.set_root_dir(root_dir.path());
    initializer.set_listener(TcpListener::bind(addr).await.unwrap());

    initializer.init_dir().await.unwrap();

    let _server_handler = tokio::spawn(initializer.run_kvsd(pending::<()>()));

    // TODO: retry
    let mut client = kvsd::client::tcp::UnauthenticatedClient::insecure_from_addr(addr.0, addr.1)
        .await
        .unwrap()
        .authenticate("test", "test")
        .await
        .unwrap();

    // Ping
    let ping_duration = client.ping().await.unwrap();
    assert!(ping_duration.num_nanoseconds().unwrap() > 0);

    Ok(client)
}

pub fn temp_dir() -> tempdir::TempDir {
    tempdir::TempDir::new("synd_term").unwrap()
}
