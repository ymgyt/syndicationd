use ratatui::backend::TestBackend;
use synd_api::{client::github::GithubClient, serve::auth::Authenticator};
use synd_term::terminal::Terminal;

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
pub async fn serve_api(mock_port: u16) -> anyhow::Result<()> {
    let github_endpoint: &'static str =
        format!("http://localhost:{mock_port}/github/graphql").leak();
    let github_client = GithubClient::new()?.with_endpoint(&github_endpoint);
    let authenticator = Authenticator::new()?.with_client(github_client);

    Ok(())
}
