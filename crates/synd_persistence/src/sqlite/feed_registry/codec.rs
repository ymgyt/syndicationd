use std::str::FromStr;

use sqlx::{Row, sqlite::SqliteRow};
use synd_feed::types::{Category, FeedUrl, Requirement};
use synd_registry::{
    RegistryDbError, RegistryDbResult, SubscriberId, Subscription,
    crawl::{policy::CrawlPolicy, target_list::CrawlTarget},
};

pub(super) fn decode_subscription(row: &SqliteRow) -> RegistryDbResult<Subscription> {
    let subscriber_id_raw: String = row
        .try_get("subscriber_id")
        .map_err(RegistryDbError::internal)?;
    let feed_url_raw: String = row.try_get("feed_url").map_err(RegistryDbError::internal)?;
    let requirement: Option<String> = row
        .try_get("requirement")
        .map_err(RegistryDbError::internal)?;
    let category: Option<String> = row.try_get("category").map_err(RegistryDbError::internal)?;
    let policy_json: String = row
        .try_get("crawl_policy_json")
        .map_err(RegistryDbError::internal)?;
    let created_at = row
        .try_get("created_at")
        .map_err(RegistryDbError::internal)?;
    let updated_at = row
        .try_get("updated_at")
        .map_err(RegistryDbError::internal)?;

    Ok(Subscription {
        subscriber_id: SubscriberId::new(subscriber_id_raw),
        feed_url: FeedUrl::parse(&feed_url_raw).map_err(RegistryDbError::internal)?,
        requirement: requirement
            .as_deref()
            .map(Requirement::from_str)
            .transpose()
            .map_err(|err| RegistryDbError::internal(anyhow::anyhow!(err)))?,
        category: category
            .map(Category::new)
            .transpose()
            .map_err(RegistryDbError::internal)?,
        crawl_policy: decode_crawl_policy_json(&policy_json)?,
        created_at,
        updated_at,
    })
}

pub(super) fn decode_crawl_target(row: &SqliteRow) -> RegistryDbResult<CrawlTarget> {
    let feed_url_raw: String = row.try_get("feed_url").map_err(RegistryDbError::internal)?;
    let state: String = row.try_get("state").map_err(RegistryDbError::internal)?;
    let subscription_count: i64 = row
        .try_get("subscription_count")
        .map_err(RegistryDbError::internal)?;
    let policy_json: Option<String> = row
        .try_get("effective_policy_json")
        .map_err(RegistryDbError::internal)?;
    let created_at = row
        .try_get("created_at")
        .map_err(RegistryDbError::internal)?;
    let updated_at = row
        .try_get("updated_at")
        .map_err(RegistryDbError::internal)?;

    if subscription_count < 0 {
        return Err(RegistryDbError::internal(anyhow::anyhow!(
            "crawl target subscription count must be non-negative: {subscription_count}"
        )));
    }
    let subscription_count =
        usize::try_from(subscription_count).map_err(RegistryDbError::internal)?;
    let is_active = match state.as_str() {
        "active" => true,
        "inactive" => false,
        state => {
            return Err(RegistryDbError::internal(anyhow::anyhow!(
                "unknown crawl target state: {state}"
            )));
        }
    };
    let crawl_policy = match (is_active, policy_json.as_deref()) {
        (false, None) => None,
        (true, Some(policy_json)) => Some(decode_crawl_policy_json(policy_json)?),
        (false, Some(_)) => {
            return Err(RegistryDbError::internal(anyhow::anyhow!(
                "inactive crawl target must not have an effective policy"
            )));
        }
        (true, None) => {
            return Err(RegistryDbError::internal(anyhow::anyhow!(
                "active crawl target requires an effective policy"
            )));
        }
    };

    Ok(CrawlTarget {
        feed_url: FeedUrl::parse(&feed_url_raw).map_err(RegistryDbError::internal)?,
        is_active,
        subscription_count,
        crawl_policy,
        created_at,
        updated_at,
    })
}

pub(super) fn encode_crawl_policy_json(policy: CrawlPolicy) -> RegistryDbResult<String> {
    serde_json::to_string(&policy).map_err(RegistryDbError::internal)
}

fn decode_crawl_policy_json(policy_json: &str) -> RegistryDbResult<CrawlPolicy> {
    serde_json::from_str(policy_json).map_err(RegistryDbError::internal)
}
