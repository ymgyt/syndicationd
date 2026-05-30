use std::{str::FromStr, time::Duration};

use sqlx::{Row, sqlite::SqliteRow};
use synd_feed::types::{Category, FeedUrl, Requirement};
use synd_registry::{
    RegistryDbError, RegistryDbResult, SubscriberId, Subscription,
    crawl::{
        policy::{RefreshPolicy, RefreshSchedule},
        state::{FeedSnapshot, RefreshErrorKind, RefreshState},
    },
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
    let policy_kind: String = row
        .try_get("refresh_policy_kind")
        .map_err(RegistryDbError::internal)?;
    let interval_seconds: Option<i64> = row
        .try_get("refresh_interval_seconds")
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
        refresh_policy: decode_policy(&policy_kind, interval_seconds)?,
        created_at,
        updated_at,
    })
}

pub(super) fn decode_snapshot(row: &SqliteRow) -> RegistryDbResult<FeedSnapshot> {
    let feed_url_raw: String = row.try_get("feed_url").map_err(RegistryDbError::internal)?;
    Ok(FeedSnapshot {
        feed_url: FeedUrl::parse(&feed_url_raw).map_err(RegistryDbError::internal)?,
        body: row.try_get("body").map_err(RegistryDbError::internal)?,
        content_type: row
            .try_get("content_type")
            .map_err(RegistryDbError::internal)?,
        etag: row.try_get("etag").map_err(RegistryDbError::internal)?,
        last_modified: row
            .try_get("last_modified")
            .map_err(RegistryDbError::internal)?,
        fetched_at: row
            .try_get("fetched_at")
            .map_err(RegistryDbError::internal)?,
    })
}

pub(super) fn decode_refresh_state(row: &SqliteRow) -> RegistryDbResult<RefreshState> {
    let feed_url_raw: String = row.try_get("feed_url").map_err(RegistryDbError::internal)?;
    let last_error_kind: Option<String> = row
        .try_get("last_error_kind")
        .map_err(RegistryDbError::internal)?;

    Ok(RefreshState {
        feed_url: FeedUrl::parse(&feed_url_raw).map_err(RegistryDbError::internal)?,
        last_attempt_at: row
            .try_get("last_attempt_at")
            .map_err(RegistryDbError::internal)?,
        last_success_at: row
            .try_get("last_success_at")
            .map_err(RegistryDbError::internal)?,
        last_failure_at: row
            .try_get("last_failure_at")
            .map_err(RegistryDbError::internal)?,
        last_error_kind: last_error_kind
            .as_deref()
            .map(RefreshErrorKind::try_from)
            .transpose()
            .map_err(RegistryDbError::internal)?,
        last_error_message: row
            .try_get("last_error_message")
            .map_err(RegistryDbError::internal)?,
        next_refresh_after: row
            .try_get("next_refresh_after")
            .map_err(RegistryDbError::internal)?,
    })
}

pub(super) fn encode_policy(policy: RefreshPolicy) -> (&'static str, Option<i64>) {
    match policy.schedule {
        RefreshSchedule::Manual => ("manual", None),
        RefreshSchedule::Interval(interval) => {
            let seconds = i64::try_from(interval.as_secs()).unwrap_or(i64::MAX);
            ("interval", Some(seconds))
        }
    }
}

fn decode_policy(kind: &str, interval_seconds: Option<i64>) -> RegistryDbResult<RefreshPolicy> {
    match kind {
        "manual" if interval_seconds.is_none() => Ok(RefreshPolicy {
            schedule: RefreshSchedule::Manual,
        }),
        "manual" => Err(RegistryDbError::internal(anyhow::anyhow!(
            "manual refresh policy must not have interval seconds"
        ))),
        "interval" => match interval_seconds {
            Some(seconds) if seconds > 0 => Ok(RefreshPolicy {
                schedule: RefreshSchedule::Interval(
                    Duration::from_secs(u64::try_from(seconds).map_err(RegistryDbError::internal)?)
                        .try_into()
                        .map_err(RegistryDbError::internal)?,
                ),
            }),
            _ => Err(RegistryDbError::internal(anyhow::anyhow!(
                "interval refresh policy requires positive interval seconds"
            ))),
        },
        kind => Err(RegistryDbError::internal(anyhow::anyhow!(
            "unknown refresh policy kind: {kind}"
        ))),
    }
}
