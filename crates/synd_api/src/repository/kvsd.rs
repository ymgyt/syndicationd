use std::{collections::HashMap, io::ErrorKind, time::Duration};

use anyhow::Context;
use async_trait::async_trait;
use futures_util::TryFutureExt;
use kvsd::{
    Key, Value,
    client::{Api, tcp::Client},
};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::{net::TcpStream, sync::MutexGuard};

use crate::repository::{
    self, RepositoryError, SubscriptionRepository,
    subscription::RepositoryResult,
    types::{FeedAnnotations, SubscribedFeeds},
};

#[derive(Error, Debug)]
#[error("connect kvsd failed")]
pub struct ConnectKvsdFailed;

pub struct KvsdClient {
    #[allow(dead_code)]
    client: Mutex<Client<TcpStream>>,
}

impl KvsdClient {
    pub fn new(client: Client<TcpStream>) -> Self {
        Self {
            client: Mutex::new(client),
        }
    }

    pub async fn connect(
        host: impl AsRef<str>,
        port: u16,
        username: String,
        password: String,
        timeout: Duration,
    ) -> anyhow::Result<Self> {
        let handshake = async {
            let mut retry = 0;
            loop {
                match kvsd::client::tcp::UnauthenticatedClient::insecure_from_addr(&host, port)
                    .and_then(|client| client.authenticate(&username, &password))
                    .await
                    .map(Self::new)
                {
                    Ok(client) => break Ok(client),
                    Err(kvsd::KvsdError::Io(io)) if io.kind() == ErrorKind::ConnectionRefused => {
                        tracing::info!(retry, "Kvsd connection refused");
                    }
                    err => break err,
                }
                retry += 1;
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        };

        tokio::time::timeout(timeout, handshake)
            .await
            .map_err(anyhow::Error::from)
            .context(ConnectKvsdFailed)?
            .map_err(anyhow::Error::from)
            .inspect(|_| tracing::info!("Kvsd handshake successfully completed"))
    }

    async fn get<T>(
        client: &mut MutexGuard<'_, Client<TcpStream>>,
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

    async fn set<T>(
        client: &mut MutexGuard<'_, Client<TcpStream>>,
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
    #[tracing::instrument(name = "repo::put_feed_subscription", skip_all)]
    async fn put_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> RepositoryResult<()> {
        let key = Self::feed_subscription_key(&feed.user_id);

        let mut client = self.client.lock().await;
        let annotations = FeedAnnotations {
            requirement: feed.requirement,
            category: feed.category,
        };

        let feeds = if let Some(mut feeds) =
            Self::get::<SubscribedFeeds>(&mut client, key.clone()).await?
        {
            // Create case
            if !feeds.urls.contains(&feed.url) {
                feeds.urls.insert(0, feed.url.clone());
            };
            if feeds.annotations.is_none() {
                feeds.annotations = Some(HashMap::new());
            };
            feeds
                .annotations
                .as_mut()
                .map(|m| m.insert(feed.url, annotations));
            feeds
        } else {
            // for investigating data loss
            tracing::warn!(
                enduser.id = feed.user_id,
                feed_url = %feed.url,
                "SubscribedFeeds not found"
            );

            let mut metadata = HashMap::new();
            metadata.insert(feed.url.clone(), annotations);
            SubscribedFeeds {
                urls: vec![feed.url.clone()],
                annotations: Some(metadata),
            }
        };

        Self::set(&mut client, key, feeds).await
    }

    #[tracing::instrument(name = "repo::delete_feed_subscription", skip_all)]
    async fn delete_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> RepositoryResult<()> {
        let key = Self::feed_subscription_key(&feed.user_id);

        let mut client = self.client.lock().await;

        let Some(mut feeds) = Self::get::<SubscribedFeeds>(&mut client, key.clone()).await? else {
            return Ok(());
        };

        feeds.urls.retain(|url| url != &feed.url);
        feeds.annotations.as_mut().map(|m| m.remove(&feed.url));

        Self::set(&mut client, key, feeds).await
    }

    #[tracing::instrument(name = "repo::fetch_subscribed_feed_urls", skip_all)]
    async fn fetch_subscribed_feeds(&self, user_id: &str) -> RepositoryResult<SubscribedFeeds> {
        let key = Self::feed_subscription_key(user_id);

        let mut client = self.client.lock().await;
        let Some(feeds) = Self::get::<SubscribedFeeds>(&mut client, key).await? else {
            return Ok(SubscribedFeeds::default());
        };
        Ok(feeds)
    }
}
