use std::{collections::HashMap, path::Path, time::Duration};

use async_trait::async_trait;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use tracing::info;

use crate::repository::{
    FeedAnnotations, FeedSubscription, RepositoryError, SubscribedFeeds, SubscriptionRepository,
    subscription::RepositoryResult,
};

pub struct SqliteSubscriptionRepository {
    pool: SqlitePool,
}

enum FileMode {
    Existing,
    CreateIfMissing,
}

impl SqliteSubscriptionRepository {
    pub async fn open(db_path: impl AsRef<Path>) -> Result<Self, RepositoryError> {
        Self::open_file(db_path, FileMode::Existing).await
    }

    pub async fn create_or_open(db_path: impl AsRef<Path>) -> Result<Self, RepositoryError> {
        Self::open_file(db_path, FileMode::CreateIfMissing).await
    }

    pub async fn migrate(&self) -> Result<(), RepositoryError> {
        info!("Run migrations...");
        sqlx::migrate!().run(&self.pool).await?;
        Ok(())
    }

    async fn open_file(db_path: impl AsRef<Path>, mode: FileMode) -> Result<Self, RepositoryError> {
        let opts = Self::file_options(db_path, mode);
        Self::build_pool(opts).await
    }

    fn file_options(db_path: impl AsRef<Path>, mode: FileMode) -> SqliteConnectOptions {
        Self::common_options(
            SqliteConnectOptions::new()
                .filename(db_path)
                .create_if_missing(matches!(mode, FileMode::CreateIfMissing))
                .journal_mode(SqliteJournalMode::Wal),
        )
    }

    fn common_options(opts: SqliteConnectOptions) -> SqliteConnectOptions {
        opts.foreign_keys(true).busy_timeout(Duration::from_secs(5))
    }

    async fn build_pool(opts: SqliteConnectOptions) -> Result<Self, RepositoryError> {
        info!(?opts, "Connecting to sqlite...");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;

        Ok(Self { pool })
    }

    #[cfg(test)]
    pub async fn in_memory() -> Result<Self, RepositoryError> {
        Self::build_pool(Self::common_options(
            SqliteConnectOptions::new().in_memory(true),
        ))
        .await
    }
}

#[async_trait]
impl SubscriptionRepository for SqliteSubscriptionRepository {
    #[tracing::instrument(name = "repo::put_feed_subscription", skip_all)]
    async fn put_feed_subscription(&self, feed: FeedSubscription) -> RepositoryResult<()> {
        let feed_url = feed.url.to_string();
        let requirement = feed.requirement.map(|r| r.to_string());
        let category = feed.category.map(|c| c.to_string());

        sqlx::query!(
            r#"
            INSERT INTO subscribed_feed (user_id, url, requirement, category)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(user_id, url) DO UPDATE SET
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

    async fn delete_feed_subscription(&self, feed: FeedSubscription) -> RepositoryResult<()> {
        let affected = sqlx::query!(
            r#"
            DELETE FROM subscribed_feed
            WHERE user_id = ? AND url = ?
            "#,
            feed.user_id,
            feed.url,
        )
        .execute(&self.pool)
        .await
        .map_err(RepositoryError::internal)?
        .rows_affected();

        if affected > 0 {
            info!(url = %feed.url, "Delete subscribed feed");
        }

        Ok(())
    }

    async fn fetch_subscribed_feeds(&self, user_id: &str) -> RepositoryResult<SubscribedFeeds> {
        use synd_feed::types::{Category, FeedUrl, Requirement};
        let feeds = sqlx::query_as!(
            FeedSubscription,
            r#"
                SELECT
                    user_id,
                    url AS "url: FeedUrl",
                    requirement AS "requirement: Requirement",
                    category AS "category: Category"
                FROM subscribed_feed
                WHERE user_id = ?
                ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .fold(
            SubscribedFeeds {
                urls: Vec::new(),
                annotations: Some(HashMap::new()),
            },
            |mut feeds, feed| {
                let annot = FeedAnnotations {
                    requirement: feed.requirement,
                    category: feed.category,
                };
                feeds
                    .annotations
                    .as_mut()
                    .unwrap()
                    .insert(feed.url.clone(), annot);
                feeds.urls.push(feed.url);
                feeds
            },
        );
        tracing::info!("{feeds:?}");
        Ok(feeds)
    }
}

#[cfg(test)]
mod tests {
    use synd_feed::types::{Category, Requirement};

    use super::*;

    async fn test_db() -> SqliteSubscriptionRepository {
        let db = SqliteSubscriptionRepository::in_memory().await.unwrap();
        db.migrate().await.unwrap();
        db
    }

    #[tokio::test]
    async fn feed_subscription() -> anyhow::Result<()> {
        let db = test_db().await;
        let user_id = String::from("test1");

        let mut test_feeds = vec![
            FeedSubscription {
                user_id: user_id.clone(),
                url: "https://ymgyt.io/feed_1".try_into().unwrap(),
                requirement: Some(Requirement::Must),
                category: Some(Category::new("rust").unwrap()),
            },
            FeedSubscription {
                user_id: user_id.clone(),
                url: "https://ymgyt.io/feed_2".try_into().unwrap(),
                requirement: Some(Requirement::Should),
                category: Some(Category::new("linux").unwrap()),
            },
        ];

        let feeds = db.fetch_subscribed_feeds(&user_id).await?;
        assert!(feeds.urls.is_empty());

        // create
        {
            for feed in &test_feeds {
                db.put_feed_subscription(feed.clone()).await?;
            }
            let feeds = db.fetch_subscribed_feeds(&user_id).await?;

            insta::assert_yaml_snapshot!("create", feeds, {
                ".annotations" => insta::sorted_redaction(),
            });
        }

        // update
        {
            test_feeds[0].requirement = Some(Requirement::May);
            test_feeds[0].category = Some(Category::new("foo").unwrap());
            db.put_feed_subscription(test_feeds[0].clone()).await?;

            let feeds = db.fetch_subscribed_feeds(&user_id).await?;
            insta::assert_yaml_snapshot!("update", feeds, {
                ".annotations" => insta::sorted_redaction(),
            });
        }

        // delete
        {
            db.delete_feed_subscription(test_feeds[0].clone()).await?;
            let feeds = db.fetch_subscribed_feeds(&user_id).await?;
            insta::assert_yaml_snapshot!("delete", feeds, {
                ".annotations" => insta::sorted_redaction(),
            });
        }

        Ok(())
    }
}
