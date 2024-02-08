use async_trait::async_trait;
use kvsd::{
    client::{tcp::Client, Api},
    Key, Value,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::{net::TcpStream, sync::MutexGuard};

use crate::repository::{
    self, subscription::RepositoryResult, RepositoryError, SubscriptionRepository,
};

pub struct KvsdClient {
    #[allow(dead_code)]
    client: Mutex<Client<TcpStream>>,
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

        Ok(Self {
            client: Mutex::new(client),
        })
    }

    async fn get<'a, T>(
        client: &mut MutexGuard<'a, Client<TcpStream>>,
        key: Key,
    ) -> RepositoryResult<Option<T>>
    where
        T: TryFrom<Value>,
        T::Error: Into<RepositoryError>,
    {
        let Some(value) = client.get(key).await.map_err(RepositoryError::internal)? else {
            return Ok(None);
        };
        Ok(Some(value.try_into().map_err(Into::into)?))
    }

    async fn set<'a, T>(
        client: &mut MutexGuard<'a, Client<TcpStream>>,
        key: Key,
        value: T,
    ) -> RepositoryResult<()>
    where
        T: TryInto<Value>,
        T::Error: Into<RepositoryError>,
    {
        let value = value.try_into().map_err(Into::into)?;
        client.set(key, value).await?;
        Ok(())
    }

    fn feed_subscription_key(user_id: &str) -> Key {
        let key = format!(
            "{prefix}/subscription/{user_id}",
            prefix = Self::key_prefix()
        );
        Key::new(key).expect("Invalid key")
    }

    fn key_prefix() -> &'static str {
        "/synd_api/v1"
    }
}

#[async_trait]
impl SubscriptionRepository for KvsdClient {
    async fn put_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> RepositoryResult<()> {
        let key = Self::feed_subscription_key(&feed.user_id);

        let mut client = self.client.lock().await;

        let urls = if let Some(mut urls) =
            Self::get::<SubscriptionUrls>(&mut client, key.clone()).await?
        {
            urls.urls.insert(0, feed.url);
            urls
        } else {
            SubscriptionUrls {
                urls: vec![feed.url],
            }
        };

        Self::set(&mut client, key, urls).await
    }

    async fn delete_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> RepositoryResult<()> {
        let key = Self::feed_subscription_key(&feed.user_id);

        let mut client = self.client.lock().await;

        let Some(mut urls) = Self::get::<SubscriptionUrls>(&mut client, key.clone()).await? else {
            return Ok(());
        };

        urls.urls.retain(|url| url != &feed.url);

        Self::set(&mut client, key, urls).await
    }

    async fn fetch_subscribed_feed_urls(&self, user_id: &str) -> RepositoryResult<Vec<String>> {
        let key = Self::feed_subscription_key(user_id);

        let mut client = self.client.lock().await;
        let Some(urls) = Self::get::<SubscriptionUrls>(&mut client, key).await? else {
            return Ok(Vec::new());
        };
        Ok(urls.urls)
    }
}

#[derive(Serialize, Deserialize)]
struct SubscriptionUrls {
    urls: Vec<String>,
}

impl TryFrom<Value> for SubscriptionUrls {
    type Error = RepositoryError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_slice(&value).map_err(RepositoryError::internal)
    }
}

impl TryFrom<SubscriptionUrls> for Value {
    type Error = RepositoryError;

    fn try_from(value: SubscriptionUrls) -> Result<Self, Self::Error> {
        let value = serde_json::to_vec(&value).map_err(RepositoryError::internal)?;
        Ok(Value::new(value).unwrap())
    }
}
