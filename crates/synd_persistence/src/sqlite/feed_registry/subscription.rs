use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::{Category, FeedUrl, Requirement};
use synd_registry::{
    FeedSubscriptionAttrs, RegistryDbResult, SubscriberId, Subscription, SubscriptionKey,
    SubscriptionStore,
    crawl::target_list::{FeedEndpointSubscription, FeedEndpointSubscriptionSet},
    query::{Subscriptions, SubscriptionsQuery},
};

use super::{
    SqliteRegistryTx, codec,
    error::{DecodeResultExt, IntoDbResult, SqliteResult},
    feed_endpoint,
    pagination::PageLimit,
};

const SUBSCRIPTION_SELECT_COLUMNS: &str = r#"
s.subscriber_id AS subscriber_id,
e.url AS feed_url,
s.requirement AS requirement,
s.category AS category,
s.crawl_policy_json AS crawl_policy_json,
s.created_at AS created_at,
s.updated_at AS updated_at
"#;

async fn upsert(
    tx: &mut Transaction<'_, Sqlite>,
    subscription: &SubscriptionKey,
    attrs: FeedSubscriptionAttrs,
    now: DateTime<Utc>,
) -> SqliteResult<()> {
    let requirement = attrs.requirement.map(|r| r.to_string());
    let category = attrs.category.map(|c| c.to_string());
    let policy_json = codec::encode_crawl_policy_json(attrs.crawl_policy)?;
    let feed_endpoint_pk = feed_endpoint::upsert(tx, &subscription.feed_url, now, now).await?;

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
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn delete(
    tx: &mut Transaction<'_, Sqlite>,
    subscriber_id: &SubscriberId,
    feed_url: &FeedUrl,
) -> SqliteResult<()> {
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
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn contains(
    tx: &mut Transaction<'_, Sqlite>,
    subscriber_id: &SubscriberId,
    feed_url: &FeedUrl,
) -> SqliteResult<bool> {
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
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.is_some())
}

async fn list(
    tx: &mut Transaction<'_, Sqlite>,
    query: SubscriptionsQuery,
) -> SqliteResult<Subscriptions> {
    let page_limit = PageLimit::new(query.first);
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
            .bind(page_limit.sql_limit())
            .fetch_all(&mut **tx)
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
            .bind(page_limit.sql_limit())
            .fetch_all(&mut **tx)
            .await
    }?;

    let mut nodes = rows
        .into_iter()
        .map(SubscriptionRow::into_subscription)
        .collect::<SqliteResult<Vec<_>>>()?;
    let has_next_page = page_limit.truncate_overfetch(&mut nodes);
    let end_cursor = nodes.last().map(|sub| sub.feed_url.to_string());

    Ok(Subscriptions::from_subscriptions(
        nodes,
        has_next_page,
        end_cursor,
    ))
}

async fn load_for_endpoint(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
) -> SqliteResult<FeedEndpointSubscriptionSet> {
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
    .fetch_all(&mut **tx)
    .await?;

    let subscriptions = rows
        .into_iter()
        .map(FeedEndpointSubscriptionRow::into_subscription)
        .collect::<SqliteResult<Vec<_>>>()?;

    Ok(FeedEndpointSubscriptionSet::new(
        feed_url.clone(),
        subscriptions,
    ))
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

impl SubscriptionRow {
    fn into_subscription(self) -> SqliteResult<Subscription> {
        Ok(Subscription {
            subscriber_id: SubscriberId::new(self.subscriber_id),
            feed_url: FeedUrl::parse(&self.feed_url).decode()?,
            requirement: self
                .requirement
                .as_deref()
                .map(Requirement::from_str)
                .transpose()
                .decode()?,
            category: self.category.map(Category::new).transpose().decode()?,
            crawl_policy: codec::decode_crawl_policy_json(&self.crawl_policy_json)?,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct FeedEndpointSubscriptionRow {
    subscriber_id: String,
    feed_url: String,
    crawl_policy_json: String,
}

impl FeedEndpointSubscriptionRow {
    fn into_subscription(self) -> SqliteResult<FeedEndpointSubscription> {
        let subscription = SubscriptionKey::new(
            SubscriberId::new(self.subscriber_id),
            FeedUrl::parse(&self.feed_url).decode()?,
        );

        Ok(FeedEndpointSubscription::new(
            subscription,
            codec::decode_crawl_policy_json(&self.crawl_policy_json)?,
        ))
    }
}

impl SubscriptionStore for SqliteRegistryTx<'_> {
    async fn upsert_subscription(
        &mut self,
        subscription: &SubscriptionKey,
        attrs: FeedSubscriptionAttrs,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        upsert(&mut self.tx, subscription, attrs, now).await.db()
    }

    async fn delete_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<()> {
        delete(&mut self.tx, subscriber_id, feed_url).await.db()
    }

    async fn has_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<bool> {
        contains(&mut self.tx, subscriber_id, feed_url).await.db()
    }

    async fn list_subscriptions(
        &mut self,
        query: SubscriptionsQuery,
    ) -> RegistryDbResult<Subscriptions> {
        list(&mut self.tx, query).await.db()
    }

    async fn load_endpoint_subscriptions(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<FeedEndpointSubscriptionSet> {
        load_for_endpoint(&mut self.tx, feed_url).await.db()
    }
}

#[cfg(test)]
mod tests;
