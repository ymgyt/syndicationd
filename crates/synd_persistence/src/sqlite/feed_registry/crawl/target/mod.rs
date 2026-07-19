use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    CrawlTargetStore, RegistryDbResult,
    crawl::{
        due::CrawlDueInput,
        target_list::{CrawlTarget, CrawlTargetState},
    },
};

use super::{
    super::{
        SqliteRegistryTx, codec,
        error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
        feed,
    },
    state::CrawlStateRow,
};

async fn upsert(tx: &mut Transaction<'_, Sqlite>, target: &CrawlTarget) -> SqliteResult<()> {
    let feed_pk = feed::resolve_pk(tx, &target.feed_url).await?;
    let state = CrawlTargetStateRow::try_from(&target.state)?;

    // A pending manual request belongs to the request/completion lifecycle
    // and is preserved across target upserts.
    sqlx::query(
        r#"
            INSERT INTO crawl_target (
                feed_pk,
                state,
                effective_policy_json
            )
            VALUES (?, ?, ?)
            ON CONFLICT(feed_pk) DO UPDATE SET
                state = excluded.state,
                effective_policy_json = excluded.effective_policy_json
            "#,
    )
    .bind(feed_pk)
    .bind(state.state.to_string())
    .bind(state.effective_policy_json.as_deref())
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn load(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
) -> SqliteResult<Option<CrawlTarget>> {
    let row = sqlx::query_as::<_, CrawlTargetRow>(
        r#"
            SELECT
                f.url AS feed_url,
                ct.state AS state,
                ct.effective_policy_json AS effective_policy_json
            FROM crawl_target AS ct
            INNER JOIN feed AS f
                ON f.pk = ct.feed_pk
            WHERE f.url = ?
            "#,
    )
    .bind(feed_url.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    row.map(CrawlTargetRow::into_target).transpose()
}

const DUE_INPUT_SELECT: &str = r#"
SELECT
    f.url AS feed_url,
    ct.effective_policy_json AS effective_policy_json,
    ct.manual_requested_at AS manual_requested_at,
    cs.last_started_at,
    cs.last_finished_at,
    cs.last_http_status,
    cs.last_error_kind,
    cs.failure_streak,
    cs.retry_after,
    cs.etag,
    cs.last_modified
FROM crawl_target AS ct
INNER JOIN feed AS f
    ON f.pk = ct.feed_pk
LEFT JOIN crawl_state AS cs
    ON cs.feed_pk = ct.feed_pk
WHERE ct.state = 'active'
"#;

async fn load_due_input(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
) -> SqliteResult<Option<CrawlDueInput>> {
    let sql = format!("{DUE_INPUT_SELECT} AND f.url = ?");
    let row = sqlx::query_as::<_, CrawlDueInputRow>(&sql)
        .bind(feed_url.as_str())
        .fetch_optional(&mut **tx)
        .await?;

    row.map(CrawlDueInputRow::into_input).transpose()
}

async fn list_due_inputs(tx: &mut Transaction<'_, Sqlite>) -> SqliteResult<Vec<CrawlDueInput>> {
    let sql = format!("{DUE_INPUT_SELECT} ORDER BY f.url");
    let rows = sqlx::query_as::<_, CrawlDueInputRow>(&sql)
        .fetch_all(&mut **tx)
        .await?;

    rows.into_iter().map(CrawlDueInputRow::into_input).collect()
}

async fn set_manual_request(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    requested_at: DateTime<Utc>,
) -> SqliteResult<()> {
    sqlx::query(
        r#"
            UPDATE crawl_target
            SET manual_requested_at = ?
            WHERE feed_pk = (SELECT pk FROM feed WHERE url = ?)
              AND manual_requested_at IS NULL
            "#,
    )
    .bind(requested_at)
    .bind(feed_url.as_str())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn clear_manual_request(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    served_by_crawl_started_at: DateTime<Utc>,
) -> SqliteResult<()> {
    sqlx::query(
        r#"
            UPDATE crawl_target
            SET manual_requested_at = NULL
            WHERE feed_pk = (SELECT pk FROM feed WHERE url = ?)
              AND manual_requested_at <= ?
            "#,
    )
    .bind(feed_url.as_str())
    .bind(served_by_crawl_started_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
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
    effective_policy_json: Option<String>,
}

impl TryFrom<&CrawlTargetState> for CrawlTargetStateRow {
    type Error = SqliteError;

    fn try_from(state: &CrawlTargetState) -> Result<Self, Self::Error> {
        match state {
            CrawlTargetState::Active { effective_policy } => Ok(Self {
                state: CrawlTargetStateDb::Active,
                effective_policy_json: Some(codec::encode_crawl_policy_json(*effective_policy)?),
            }),
            CrawlTargetState::Inactive => Ok(Self {
                state: CrawlTargetStateDb::Inactive,
                effective_policy_json: None,
            }),
        }
    }
}

impl TryFrom<CrawlTargetStateRow> for CrawlTargetState {
    type Error = SqliteError;

    fn try_from(row: CrawlTargetStateRow) -> Result<Self, Self::Error> {
        match (row.state, row.effective_policy_json) {
            (CrawlTargetStateDb::Active, Some(policy_json)) => Ok(CrawlTargetState::Active {
                effective_policy: codec::decode_crawl_policy_json(&policy_json)?,
            }),
            (CrawlTargetStateDb::Active, None) => Err(SqliteError::decode_message(
                "active crawl target requires an effective policy",
            )),
            (CrawlTargetStateDb::Inactive, None) => Ok(CrawlTargetState::Inactive),
            (CrawlTargetStateDb::Inactive, Some(_)) => Err(SqliteError::decode_message(
                "inactive crawl target must not have an effective policy",
            )),
        }
    }
}

#[derive(sqlx::FromRow)]
struct CrawlTargetRow {
    feed_url: String,
    state: String,
    effective_policy_json: Option<String>,
}

impl CrawlTargetRow {
    fn into_target(self) -> SqliteResult<CrawlTarget> {
        let state = CrawlTargetState::try_from(CrawlTargetStateRow {
            state: self.state.parse()?,
            effective_policy_json: self.effective_policy_json,
        })?;

        Ok(CrawlTarget {
            feed_url: FeedUrl::parse(&self.feed_url).decode()?,
            state,
        })
    }
}

#[derive(sqlx::FromRow)]
struct CrawlDueInputRow {
    feed_url: String,
    effective_policy_json: Option<String>,
    manual_requested_at: Option<DateTime<Utc>>,
    last_started_at: Option<DateTime<Utc>>,
    last_finished_at: Option<DateTime<Utc>>,
    last_http_status: Option<i64>,
    last_error_kind: Option<String>,
    failure_streak: Option<i64>,
    retry_after: Option<DateTime<Utc>>,
    etag: Option<String>,
    last_modified: Option<String>,
}

impl CrawlDueInputRow {
    fn into_input(self) -> SqliteResult<CrawlDueInput> {
        let feed_url = FeedUrl::parse(&self.feed_url).decode()?;
        let Some(policy_json) = self.effective_policy_json else {
            return Err(SqliteError::decode_message(
                "active crawl target requires an effective policy",
            ));
        };
        let policy = codec::decode_crawl_policy_json(&policy_json)?;

        // The LEFT JOIN yields state columns together or not at all.
        let state = match (
            self.last_started_at,
            self.last_finished_at,
            self.failure_streak,
        ) {
            (Some(last_started_at), Some(last_finished_at), Some(failure_streak)) => Some(
                CrawlStateRow {
                    last_started_at,
                    last_finished_at,
                    last_http_status: self.last_http_status,
                    last_error_kind: self.last_error_kind,
                    failure_streak,
                    retry_after: self.retry_after,
                    etag: self.etag,
                    last_modified: self.last_modified,
                }
                .into_state(&feed_url)?,
            ),
            _ => None,
        };

        Ok(CrawlDueInput {
            feed_url,
            polling: policy.polling,
            manual_requested_at: self.manual_requested_at,
            state,
        })
    }
}

impl CrawlTargetStore for SqliteRegistryTx<'_> {
    async fn upsert_target(&mut self, target: &CrawlTarget) -> RegistryDbResult<()> {
        upsert(&mut self.tx, target).await.db()
    }

    async fn load_target(&mut self, feed_url: &FeedUrl) -> RegistryDbResult<Option<CrawlTarget>> {
        load(&mut self.tx, feed_url).await.db()
    }

    async fn load_crawl_due_input(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<CrawlDueInput>> {
        load_due_input(&mut self.tx, feed_url).await.db()
    }

    async fn list_crawl_due_inputs(&mut self) -> RegistryDbResult<Vec<CrawlDueInput>> {
        list_due_inputs(&mut self.tx).await.db()
    }

    async fn set_manual_request(
        &mut self,
        feed_url: &FeedUrl,
        requested_at: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        set_manual_request(&mut self.tx, feed_url, requested_at)
            .await
            .db()
    }

    async fn clear_manual_request(
        &mut self,
        feed_url: &FeedUrl,
        served_by_crawl_started_at: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        clear_manual_request(&mut self.tx, feed_url, served_by_crawl_started_at)
            .await
            .db()
    }
}

#[cfg(test)]
mod tests;
