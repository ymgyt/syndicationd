use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::{Category, FeedUrl, Requirement};
use synd_registry::{
    FeedSubscriptionAttrs, RegistryDbError, RegistryDbResult, SubscriberId, Subscription,
    SubscriptionKey,
    crawl::target_list::{FeedEndpointSubscription, FeedEndpointSubscriptionSet},
    query::{Subscriptions, SubscriptionsQuery},
};

use super::{codec, feed_endpoint::FeedEndpointTable};

const SUBSCRIPTION_SELECT_COLUMNS: &str = r#"
s.subscriber_id AS subscriber_id,
e.url AS feed_url,
s.requirement AS requirement,
s.category AS category,
s.crawl_policy_json AS crawl_policy_json,
s.created_at AS created_at,
s.updated_at AS updated_at
"#;

pub(super) struct FeedEndpointSubscriptionTable<'tx, 'db> {
    tx: &'tx mut Transaction<'db, Sqlite>,
}

impl<'tx, 'db> FeedEndpointSubscriptionTable<'tx, 'db> {
    pub(super) fn new(tx: &'tx mut Transaction<'db, Sqlite>) -> Self {
        Self { tx }
    }

    pub(super) async fn upsert(
        &mut self,
        subscription: &SubscriptionKey,
        attrs: FeedSubscriptionAttrs,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        let requirement = attrs.requirement.map(|r| r.to_string());
        let category = attrs.category.map(|c| c.to_string());
        let policy_json = codec::encode_crawl_policy_json(attrs.crawl_policy)?;
        let feed_endpoint_pk = {
            let mut feed_endpoint = FeedEndpointTable::new(&mut *self.tx);
            feed_endpoint.resolve_pk(&subscription.feed_url).await?
        };

        sqlx::query(
            r#"
            INSERT INTO feed_endpoint_subscription (
                subscriber_id,
                feed_endpoint_pk,
                requirement,
                category,
                crawl_policy_json,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(subscriber_id, feed_endpoint_pk) DO UPDATE SET
                requirement = excluded.requirement,
                category = excluded.category,
                crawl_policy_json = excluded.crawl_policy_json,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(subscription.subscriber_id.as_str())
        .bind(feed_endpoint_pk)
        .bind(requirement)
        .bind(category)
        .bind(policy_json)
        .bind(now)
        .bind(now)
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    pub(super) async fn delete(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<()> {
        sqlx::query(
            r#"
            DELETE FROM feed_endpoint_subscription
            WHERE subscriber_id = ?
              AND feed_endpoint_pk = (
                  SELECT pk
                  FROM feed_endpoint
                  WHERE url = ?
              )
            "#,
        )
        .bind(subscriber_id.as_str())
        .bind(feed_url.as_str())
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    pub(super) async fn contains(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<bool> {
        let row = sqlx::query(
            r#"
            SELECT 1 AS found
            FROM feed_endpoint_subscription AS s
            INNER JOIN feed_endpoint AS e
                ON e.pk = s.feed_endpoint_pk
            WHERE s.subscriber_id = ? AND e.url = ?
            LIMIT 1
            "#,
        )
        .bind(subscriber_id.as_str())
        .bind(feed_url.as_str())
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(row.is_some())
    }

    pub(super) async fn list(
        &mut self,
        query: SubscriptionsQuery,
    ) -> RegistryDbResult<Subscriptions> {
        let first = i64::try_from(query.first.saturating_add(1)).unwrap_or(i64::MAX);
        let rows = if let Some(after) = query.after {
            let sql = format!(
                r#"
                SELECT {SUBSCRIPTION_SELECT_COLUMNS}
                FROM feed_endpoint_subscription AS s
                INNER JOIN feed_endpoint AS e
                    ON e.pk = s.feed_endpoint_pk
                WHERE s.subscriber_id = ? AND e.url > ?
                ORDER BY e.url
                LIMIT ?
                "#
            );
            sqlx::query_as::<_, SubscriptionRow>(&sql)
                .bind(query.subscriber_id.as_str())
                .bind(after)
                .bind(first)
                .fetch_all(&mut **self.tx)
                .await
        } else {
            let sql = format!(
                r#"
                SELECT {SUBSCRIPTION_SELECT_COLUMNS}
                FROM feed_endpoint_subscription AS s
                INNER JOIN feed_endpoint AS e
                    ON e.pk = s.feed_endpoint_pk
                WHERE s.subscriber_id = ?
                ORDER BY e.url
                LIMIT ?
                "#
            );
            sqlx::query_as::<_, SubscriptionRow>(&sql)
                .bind(query.subscriber_id.as_str())
                .bind(first)
                .fetch_all(&mut **self.tx)
                .await
        }
        .map_err(RegistryDbError::internal)?;

        let mut nodes = rows
            .into_iter()
            .map(Subscription::try_from)
            .collect::<RegistryDbResult<Vec<_>>>()?;
        let has_next_page = nodes.len() > query.first;
        if has_next_page {
            nodes.truncate(query.first);
        }
        let end_cursor = nodes.last().map(|sub| sub.feed_url.to_string());

        Ok(Subscriptions::from_subscriptions(
            nodes,
            has_next_page,
            end_cursor,
        ))
    }

    pub(super) async fn load_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<FeedEndpointSubscriptionSet> {
        let rows = sqlx::query_as::<_, FeedEndpointSubscriptionRow>(
            r#"
            SELECT
                s.subscriber_id AS subscriber_id,
                e.url AS feed_url,
                s.crawl_policy_json AS crawl_policy_json
            FROM feed_endpoint_subscription AS s
            INNER JOIN feed_endpoint AS e
                ON e.pk = s.feed_endpoint_pk
            WHERE e.url = ?
            ORDER BY s.subscriber_id
            "#,
        )
        .bind(feed_url.as_str())
        .fetch_all(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let subscriptions = rows
            .into_iter()
            .map(FeedEndpointSubscription::try_from)
            .collect::<RegistryDbResult<Vec<_>>>()?;

        Ok(FeedEndpointSubscriptionSet::new(
            feed_url.clone(),
            subscriptions,
        ))
    }
}

#[derive(sqlx::FromRow)]
struct SubscriptionRow {
    subscriber_id: String,
    feed_url: String,
    requirement: Option<String>,
    category: Option<String>,
    crawl_policy_json: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<SubscriptionRow> for Subscription {
    type Error = RegistryDbError;

    fn try_from(row: SubscriptionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            subscriber_id: SubscriberId::new(row.subscriber_id),
            feed_url: FeedUrl::parse(&row.feed_url).map_err(RegistryDbError::internal)?,
            requirement: row
                .requirement
                .as_deref()
                .map(Requirement::from_str)
                .transpose()
                .map_err(|err| RegistryDbError::internal(anyhow::anyhow!(err)))?,
            category: row
                .category
                .map(Category::new)
                .transpose()
                .map_err(RegistryDbError::internal)?,
            crawl_policy: codec::decode_crawl_policy_json(&row.crawl_policy_json)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct FeedEndpointSubscriptionRow {
    subscriber_id: String,
    feed_url: String,
    crawl_policy_json: String,
}

impl TryFrom<FeedEndpointSubscriptionRow> for FeedEndpointSubscription {
    type Error = RegistryDbError;

    fn try_from(row: FeedEndpointSubscriptionRow) -> Result<Self, Self::Error> {
        let subscription = SubscriptionKey::new(
            SubscriberId::new(row.subscriber_id),
            FeedUrl::parse(&row.feed_url).map_err(RegistryDbError::internal)?,
        );

        Ok(Self::new(
            subscription,
            codec::decode_crawl_policy_json(&row.crawl_policy_json)?,
        ))
    }
}
