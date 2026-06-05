use std::{fmt, num::NonZeroUsize, str::FromStr};

use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    RegistryDbError, RegistryDbResult,
    crawl::target_list::{CrawlTarget, CrawlTargetState},
};

use super::{codec, feed_endpoint::FeedEndpointTable};

pub(super) struct CrawlTargetTable<'tx, 'db> {
    tx: &'tx mut Transaction<'db, Sqlite>,
}

impl<'tx, 'db> CrawlTargetTable<'tx, 'db> {
    pub(super) fn new(tx: &'tx mut Transaction<'db, Sqlite>) -> Self {
        Self { tx }
    }

    pub(super) async fn upsert(&mut self, target: &CrawlTarget) -> RegistryDbResult<()> {
        let feed_endpoint_pk = {
            let mut feed_endpoint = FeedEndpointTable::new(&mut *self.tx);
            feed_endpoint.resolve_pk(&target.feed_url).await?
        };
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
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    pub(super) async fn load_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<CrawlTarget>> {
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
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        row.map(CrawlTarget::try_from).transpose()
    }
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
    type Err = RegistryDbError;

    fn from_str(state: &str) -> Result<Self, Self::Err> {
        match state {
            Self::ACTIVE => Ok(Self::Active),
            Self::INACTIVE => Ok(Self::Inactive),
            state => Err(RegistryDbError::internal(anyhow::anyhow!(
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
    type Error = RegistryDbError;

    fn try_from(state: &CrawlTargetState) -> Result<Self, Self::Error> {
        match state {
            CrawlTargetState::Active {
                subscription_count,
                effective_policy,
            } => Ok(Self {
                state: CrawlTargetStateDb::Active,
                subscription_count: i64::try_from(subscription_count.get())
                    .map_err(RegistryDbError::internal)?,
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
    type Error = RegistryDbError;

    fn try_from(row: CrawlTargetStateRow) -> Result<Self, Self::Error> {
        if row.subscription_count < 0 {
            return Err(RegistryDbError::internal(anyhow::anyhow!(
                "crawl target subscription count must be non-negative: {}",
                row.subscription_count
            )));
        }
        let subscription_count =
            usize::try_from(row.subscription_count).map_err(RegistryDbError::internal)?;

        match (row.state, subscription_count, row.effective_policy_json) {
            (CrawlTargetStateDb::Active, 0, _) => Err(RegistryDbError::internal(anyhow::anyhow!(
                "active crawl target subscription count must be positive"
            ))),
            (CrawlTargetStateDb::Active, subscription_count, Some(policy_json)) => {
                Ok(CrawlTargetState::Active {
                    subscription_count: NonZeroUsize::new(subscription_count)
                        .expect("active subscription count checked above"),
                    effective_policy: codec::decode_crawl_policy_json(&policy_json)?,
                })
            }
            (CrawlTargetStateDb::Active, _, None) => Err(RegistryDbError::internal(
                anyhow::anyhow!("active crawl target requires an effective policy"),
            )),
            (CrawlTargetStateDb::Inactive, 0, None) => Ok(CrawlTargetState::Inactive),
            (CrawlTargetStateDb::Inactive, _, Some(_)) => Err(RegistryDbError::internal(
                anyhow::anyhow!("inactive crawl target must not have an effective policy"),
            )),
            (CrawlTargetStateDb::Inactive, subscription_count, None) => {
                Err(RegistryDbError::internal(anyhow::anyhow!(
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

impl TryFrom<CrawlTargetRow> for CrawlTarget {
    type Error = RegistryDbError;

    fn try_from(row: CrawlTargetRow) -> Result<Self, Self::Error> {
        let state = CrawlTargetState::try_from(CrawlTargetStateRow {
            state: row.state.parse()?,
            subscription_count: row.subscription_count,
            effective_policy_json: row.effective_policy_json,
        })?;

        Ok(Self {
            feed_url: FeedUrl::parse(&row.feed_url).map_err(RegistryDbError::internal)?,
            state,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}
