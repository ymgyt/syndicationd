use std::{future::pending, time::Duration};

use futures_util::TryFutureExt;
use tokio::net::{TcpListener, TcpStream};

pub async fn run_kvsd(
    kvsd_host: String,
    kvsd_port: u16,
    kvsd_username: String,
    kvsd_password: String,
) -> anyhow::Result<kvsd::client::tcp::Client<TcpStream>> {
    let root_dir = temp_dir();
    let mut config = kvsd::config::Config::default();

    // Setup user credential.
    config.kvsd.users = vec![kvsd::core::UserEntry {
        username: kvsd_username,
        password: kvsd_password,
    }];
    config.server.set_disable_tls(&mut Some(true));

    // Test Server listen addr
    let addr = (kvsd_host, kvsd_port);

    let mut initializer = kvsd::config::Initializer::from_config(config);

    initializer.set_root_dir(root_dir.path());
    initializer.set_listener(TcpListener::bind(addr.clone()).await.unwrap());

    initializer.init_dir().await.unwrap();

    let _server_handler = tokio::spawn(initializer.run_kvsd(pending::<()>()));

    let handshake = async {
        loop {
            match kvsd::client::tcp::UnauthenticatedClient::insecure_from_addr(&addr.0, addr.1)
                .and_then(|client| client.authenticate("test", "test"))
                .await
            {
                Ok(client) => break client,
                Err(_) => tokio::time::sleep(Duration::from_millis(500)).await,
            }
        }
    };

    let client = tokio::time::timeout(Duration::from_secs(5), handshake).await?;

    Ok(client)
}

pub fn temp_dir() -> tempfile::TempDir {
    tempfile::TempDir::new().unwrap()
}
