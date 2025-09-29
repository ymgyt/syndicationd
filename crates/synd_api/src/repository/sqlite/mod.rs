use std::path::Path;

use async_trait::async_trait;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use tracing::info;

use crate::repository::{
    self, RepositoryError, SubscriptionRepository, subscription::RepositoryResult,
    types::SubscribedFeeds,
};

pub struct DbPool {
    pool: SqlitePool,
}

impl DbPool {
    pub async fn connect(db_path: impl AsRef<Path>) -> Result<Self, RepositoryError> {
        let opts = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .foreign_keys(true);

        Self::do_connect(opts).await
    }

    pub async fn migrate(&self) -> Result<(), RepositoryError> {
        info!("Run migrations...");
        sqlx::migrate!().run(&self.pool).await?;
        Ok(())
    }

    async fn do_connect(opts: SqliteConnectOptions) -> Result<Self, RepositoryError> {
        info!(?opts, "Connecting to sqlite...");
        let pool = SqlitePool::connect_with(opts).await?;

        // TODO: configure pool options

        Ok(DbPool { pool })
    }

    #[cfg(test)]
    pub async fn in_memory() -> Result<Self, RepositoryError> {
        use sqlx::sqlite::SqlitePoolOptions;

        let opts = SqliteConnectOptions::new()
            .in_memory(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        Ok(DbPool { pool })
    }
}

#[async_trait]
impl SubscriptionRepository for DbPool {
    #[tracing::instrument(name = "repo::put_feed_subscription", skip_all)]
    async fn put_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> RepositoryResult<()> {
        let feed_url = feed.url.to_string();
        let requirement = feed.requirement.map(|r| r.to_string());
        let category = feed.category.map(|c| c.to_string());

        sqlx::query!(
            r#"
            INSERT INTO subscribed_feeds (user_id, feed_url, requirement, category)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(user_id, feed_url) DO UPDATE SET
                requirement = excluded.requirement,
                category = excluded.category
            "#,
            feed.user_id,
            feed_url,
            requirement,
            category,
        )
        .execute(&self.pool)
        .await
        .map_err(RepositoryError::internal)?;

        Ok(())
    }

    async fn delete_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> RepositoryResult<()> {
        todo!()
    }

    async fn fetch_subscribed_feeds(&self, user_id: &str) -> RepositoryResult<SubscribedFeeds> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use synd_feed::types::{Category, Requirement};

    use super::*;

    async fn test_db() -> DbPool {
        let db = DbPool::in_memory().await.unwrap();
        db.migrate().await.unwrap();
        db
    }

    #[tokio::test]
    async fn feed_subscription() -> anyhow::Result<()> {
        let db = test_db().await;
        db.put_feed_subscription(repository::types::FeedSubscription {
            user_id: String::from("me"),
            url: "https://ymgyt.io/feed".try_into().unwrap(),
            requirement: Some(Requirement::Must),
            category: Some(Category::new("rust").unwrap()),
        })
        .await?;

        Ok(())
    }
}
