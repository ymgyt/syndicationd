use kvsd::client::tcp::Client;
use tokio::net::TcpStream;

pub struct KvsdClient {
    #[allow(dead_code)]
    client: Client<TcpStream>,
}

impl KvsdClient {
    pub async fn connect(
        host: impl AsRef<str>,
        port: u16,
        username: String,
        password: String,
    ) -> anyhow::Result<Self> {
        let client =
            kvsd::client::tcp::UnauthenticatedClient::insecure_from_addr(host, port).await?;

        let client = client.authenticate(username, password).await?;

        Ok(Self { client })
    }
}
