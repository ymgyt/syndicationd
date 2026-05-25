use std::{str::FromStr, time::Duration};

use sqlx::{Row, sqlite::SqliteRow};
use synd_feed::types::{Category, FeedUrl, Requirement};
use synd_registry::{
    StoreError, StoreResult,
    model::{
        FeedSnapshot, FeedSubscription, RefreshErrorKind, RefreshPolicy, RefreshSchedule,
        RefreshState, SubscriberId,
    },
};

pub(super) fn decode_subscription(row: &SqliteRow) -> StoreResult<FeedSubscription> {
    let subscriber_id: String = row.try_get("subscriber_id").map_err(StoreError::internal)?;
    let feed_url: String = row.try_get("feed_url").map_err(StoreError::internal)?;
    let requirement: Option<String> = row.try_get("requirement").map_err(StoreError::internal)?;
    let category: Option<String> = row.try_get("category").map_err(StoreError::internal)?;
    let policy_kind: String = row
        .try_get("refresh_policy_kind")
        .map_err(StoreError::internal)?;
    let interval_seconds: Option<i64> = row
        .try_get("refresh_interval_seconds")
        .map_err(StoreError::internal)?;
    let created_at = row.try_get("created_at").map_err(StoreError::internal)?;
    let updated_at = row.try_get("updated_at").map_err(StoreError::internal)?;

    Ok(FeedSubscription {
        subscriber_id: SubscriberId::new(subscriber_id),
        feed_url: FeedUrl::parse(&feed_url).map_err(StoreError::internal)?,
        requirement: requirement
            .as_deref()
            .map(Requirement::from_str)
            .transpose()
            .map_err(|err| StoreError::internal(anyhow::anyhow!(err)))?,
        category: category
            .map(Category::new)
            .transpose()
            .map_err(StoreError::internal)?,
        refresh_policy: decode_policy(&policy_kind, interval_seconds)?,
        created_at,
        updated_at,
    })
}

pub(super) fn decode_snapshot(row: &SqliteRow) -> StoreResult<FeedSnapshot> {
    let feed_url: String = row.try_get("feed_url").map_err(StoreError::internal)?;
    Ok(FeedSnapshot {
        feed_url: FeedUrl::parse(&feed_url).map_err(StoreError::internal)?,
        body: row.try_get("body").map_err(StoreError::internal)?,
        content_type: row.try_get("content_type").map_err(StoreError::internal)?,
        etag: row.try_get("etag").map_err(StoreError::internal)?,
        last_modified: row.try_get("last_modified").map_err(StoreError::internal)?,
        fetched_at: row.try_get("fetched_at").map_err(StoreError::internal)?,
    })
}

pub(super) fn decode_refresh_state(row: &SqliteRow) -> StoreResult<RefreshState> {
    let feed_url: String = row.try_get("feed_url").map_err(StoreError::internal)?;
    let last_error_kind: Option<String> = row
        .try_get("last_error_kind")
        .map_err(StoreError::internal)?;

    Ok(RefreshState {
        feed_url: FeedUrl::parse(&feed_url).map_err(StoreError::internal)?,
        last_attempt_at: row
            .try_get("last_attempt_at")
            .map_err(StoreError::internal)?,
        last_success_at: row
            .try_get("last_success_at")
            .map_err(StoreError::internal)?,
        last_failure_at: row
            .try_get("last_failure_at")
            .map_err(StoreError::internal)?,
        last_error_kind: last_error_kind
            .as_deref()
            .map(RefreshErrorKind::try_from)
            .transpose()
            .map_err(StoreError::internal)?,
        last_error_message: row
            .try_get("last_error_message")
            .map_err(StoreError::internal)?,
        next_refresh_after: row
            .try_get("next_refresh_after")
            .map_err(StoreError::internal)?,
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

fn decode_policy(kind: &str, interval_seconds: Option<i64>) -> StoreResult<RefreshPolicy> {
    match kind {
        "manual" if interval_seconds.is_none() => Ok(RefreshPolicy {
            schedule: RefreshSchedule::Manual,
        }),
        "manual" => Err(StoreError::internal(anyhow::anyhow!(
            "manual refresh policy must not have interval seconds"
        ))),
        "interval" => match interval_seconds {
            Some(seconds) if seconds > 0 => Ok(RefreshPolicy {
                schedule: RefreshSchedule::Interval(
                    Duration::from_secs(u64::try_from(seconds).map_err(StoreError::internal)?)
                        .try_into()
                        .map_err(StoreError::internal)?,
                ),
            }),
            _ => Err(StoreError::internal(anyhow::anyhow!(
                "interval refresh policy requires positive interval seconds"
            ))),
        },
        kind => Err(StoreError::internal(anyhow::anyhow!(
            "unknown refresh policy kind: {kind}"
        ))),
    }
}
