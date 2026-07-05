use std::{fmt, num::NonZeroUsize, str::FromStr};

use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    CrawlTargetStore, RegistryDbResult,
    crawl::target_list::{CrawlTarget, CrawlTargetState},
};

use super::super::{
    SqliteRegistryTx, codec,
    error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
    feed_endpoint,
};

async fn upsert(tx: &mut Transaction<'_, Sqlite>, target: &CrawlTarget) -> SqliteResult<()> {
    let feed_endpoint_pk = feed_endpoint::resolve_pk(tx, &target.feed_url).await?;
    let state = CrawlTargetStateRow::try_from(&target.state)?;

    sqlx::query(
        r#"
            INSERT INTO crawl_target (
                feed_endpoint_pk,
                state,
                subscription_count,
                effective_policy_json,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(feed_endpoint_pk) DO UPDATE SET
                state = excluded.state,
                subscription_count = excluded.subscription_count,
                effective_policy_json = excluded.effective_policy_json,
                updated_at = excluded.updated_at
            "#,
    )
    .bind(feed_endpoint_pk)
    .bind(state.state.to_string())
    .bind(state.subscription_count)
    .bind(state.effective_policy_json.as_deref())
    .bind(target.created_at)
    .bind(target.updated_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn load_for_endpoint(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
) -> SqliteResult<Option<CrawlTarget>> {
    let row = sqlx::query_as::<_, CrawlTargetRow>(
        r#"
            SELECT
                e.url AS feed_url,
                ct.state AS state,
                ct.subscription_count AS subscription_count,
                ct.effective_policy_json AS effective_policy_json,
                ct.created_at AS created_at,
                ct.updated_at AS updated_at
            FROM crawl_target AS ct
            INNER JOIN feed_endpoint AS e
                ON e.pk = ct.feed_endpoint_pk
            WHERE e.url = ?
            "#,
    )
    .bind(feed_url.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    row.map(CrawlTargetRow::into_target).transpose()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CrawlTargetStateDb {
    Active,
    Inactive,
}

impl CrawlTargetStateDb {
    const ACTIVE: &'static str = "active";
    const INACTIVE: &'static str = "inactive";
}

impl fmt::Display for CrawlTargetStateDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => f.write_str(Self::ACTIVE),
            Self::Inactive => f.write_str(Self::INACTIVE),
        }
    }
}

impl FromStr for CrawlTargetStateDb {
    type Err = SqliteError;

    fn from_str(state: &str) -> Result<Self, Self::Err> {
        match state {
            Self::ACTIVE => Ok(Self::Active),
            Self::INACTIVE => Ok(Self::Inactive),
            state => Err(SqliteError::decode_message(format!(
                "unknown crawl target state: {state}"
            ))),
        }
    }
}

struct CrawlTargetStateRow {
    state: CrawlTargetStateDb,
    subscription_count: i64,
    effective_policy_json: Option<String>,
}

impl TryFrom<&CrawlTargetState> for CrawlTargetStateRow {
    type Error = SqliteError;

    fn try_from(state: &CrawlTargetState) -> Result<Self, Self::Error> {
        match state {
            CrawlTargetState::Active {
                subscription_count,
                effective_policy,
            } => Ok(Self {
                state: CrawlTargetStateDb::Active,
                subscription_count: i64::try_from(subscription_count.get()).map_err(|_| {
                    SqliteError::decode_message("subscription count exceeds SQLite INTEGER range")
                })?,
                effective_policy_json: Some(codec::encode_crawl_policy_json(*effective_policy)?),
            }),
            CrawlTargetState::Inactive => Ok(Self {
                state: CrawlTargetStateDb::Inactive,
                subscription_count: 0,
                effective_policy_json: None,
            }),
        }
    }
}

impl TryFrom<CrawlTargetStateRow> for CrawlTargetState {
    type Error = SqliteError;

    fn try_from(row: CrawlTargetStateRow) -> Result<Self, Self::Error> {
        if row.subscription_count < 0 {
            return Err(SqliteError::decode_message(format!(
                "crawl target subscription count must be non-negative: {}",
                row.subscription_count
            )));
        }
        let subscription_count = usize::try_from(row.subscription_count).map_err(|_| {
            SqliteError::decode_message("crawl target subscription count exceeds usize range")
        })?;

        match (row.state, subscription_count, row.effective_policy_json) {
            (CrawlTargetStateDb::Active, 0, _) => Err(SqliteError::decode_message(
                "active crawl target subscription count must be positive",
            )),
            (CrawlTargetStateDb::Active, subscription_count, Some(policy_json)) => {
                Ok(CrawlTargetState::Active {
                    subscription_count: NonZeroUsize::new(subscription_count)
                        .expect("active subscription count checked above"),
                    effective_policy: codec::decode_crawl_policy_json(&policy_json)?,
                })
            }
            (CrawlTargetStateDb::Active, _, None) => Err(SqliteError::decode_message(
                "active crawl target requires an effective policy",
            )),
            (CrawlTargetStateDb::Inactive, 0, None) => Ok(CrawlTargetState::Inactive),
            (CrawlTargetStateDb::Inactive, _, Some(_)) => Err(SqliteError::decode_message(
                "inactive crawl target must not have an effective policy",
            )),
            (CrawlTargetStateDb::Inactive, subscription_count, None) => {
                Err(SqliteError::decode_message(format!(
                    "inactive crawl target subscription count must be zero: {subscription_count}"
                )))
            }
        }
    }
}

#[derive(sqlx::FromRow)]
struct CrawlTargetRow {
    feed_url: String,
    state: String,
    subscription_count: i64,
    effective_policy_json: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl CrawlTargetRow {
    fn into_target(self) -> SqliteResult<CrawlTarget> {
        let state = CrawlTargetState::try_from(CrawlTargetStateRow {
            state: self.state.parse()?,
            subscription_count: self.subscription_count,
            effective_policy_json: self.effective_policy_json,
        })?;

        Ok(CrawlTarget {
            feed_url: FeedUrl::parse(&self.feed_url).decode()?,
            state,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

impl CrawlTargetStore for SqliteRegistryTx<'_> {
    async fn upsert_target(&mut self, target: &CrawlTarget) -> RegistryDbResult<()> {
        upsert(&mut self.tx, target).await.db()
    }

    async fn load_target_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<CrawlTarget>> {
        load_for_endpoint(&mut self.tx, feed_url).await.db()
    }
}

#[cfg(test)]
mod tests;
