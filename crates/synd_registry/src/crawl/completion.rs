use chrono::{DateTime, Duration, Utc};
use synd_feed::feed::service::{
    FeedConditionalFetch, FeedFetchOutcome, FeedHttpResponse, FeedHttpStatus, FeedResponseBody,
};

use crate::{
    crawl::{
        blob::PutBlobCommand,
        job::CrawlJob,
        result::{
            CrawlFeedParseErrorDetail, CrawlFetchErrorDetail, CrawlHealth, CrawlHttpBodyDetail,
            CrawlHttpErrorKind, CrawlHttpResponseDetail, CrawlResultDetail, CrawlResultRecord,
            CrawlResultRef, CrawlState, CrawlStateError, LastCrawlResult, RecordCrawlResultCommand,
            UpsertCrawlStateCommand,
        },
    },
    db::{BlobStore, CrawlResultStore},
    error::RegistryDbResult,
    event::{CrawlJobFinishedEvent, Event},
};

/// Records the durable completion facts for one dispatched crawl.
///
/// Recording is split into two single-concern steps: [`store_detail`] persists
/// the observed payload (blobs plus the immutable result detail), and
/// [`derive_crawl_state`] purely classifies that detail into the current
/// crawl-state facts.
pub struct CrawlCompletionRecorder<'a, Tx> {
    tx: &'a mut Tx,
}

impl<'a, Tx> CrawlCompletionRecorder<'a, Tx> {
    pub fn new(tx: &'a mut Tx) -> Self {
        Self { tx }
    }
}

impl<Tx> CrawlCompletionRecorder<'_, Tx>
where
    Tx: BlobStore + CrawlResultStore + Send,
{
    pub async fn record(
        &mut self,
        job: CrawlJob,
        outcome: FeedFetchOutcome,
        previous_state: Option<CrawlState>,
        finished_at: DateTime<Utc>,
    ) -> RegistryDbResult<(CrawlCompletionRecord, Vec<Event>)> {
        let started_at = job.started_at;
        let feed_url = job.feed_url.clone();
        let finished_event = CrawlJobFinishedEvent::new(job.job_id.clone(), feed_url.clone());
        let previous_conditional = previous_state
            .as_ref()
            .map(|state| state.conditional.clone())
            .unwrap_or_default();

        let detail = self.store_detail(outcome, finished_at).await?;
        let derived =
            DerivedCrawlState::derive(&detail, started_at, finished_at, &previous_conditional);
        let summary = CrawlCompletionSummary::from_detail(&detail);

        let result_ref = self
            .tx
            .record_crawl_result(RecordCrawlResultCommand::new(
                CrawlResultRecord::new(job.job_id, feed_url.clone(), started_at, finished_at),
                detail,
            ))
            .await?;

        let health = CrawlHealth::for_last_result(&derived.last, previous_state.as_ref());
        self.tx
            .upsert_crawl_state(UpsertCrawlStateCommand::new(
                result_ref,
                feed_url,
                derived.last,
                health,
                derived.conditional,
                finished_at,
            ))
            .await?;

        Ok((
            CrawlCompletionRecord {
                result_ref,
                outcome: summary.outcome,
                http_status: summary.http_status,
                error_kind: summary.error_kind,
                health,
            },
            vec![finished_event.into()],
        ))
    }

    /// Persists the observed payload of one fetch outcome: headers and body
    /// blobs plus the immutable result detail shape. No classification
    /// happens here.
    async fn store_detail(
        &mut self,
        outcome: FeedFetchOutcome,
        created_at: DateTime<Utc>,
    ) -> RegistryDbResult<CrawlResultDetail> {
        match outcome {
            FeedFetchOutcome::Fetched(fetched) => {
                let (http, body) = self.persist_body(fetched.body, created_at).await?;
                Ok(CrawlResultDetail::Fetched { http, body })
            }
            FeedFetchOutcome::NotModified(response) => {
                let http = self.persist_response(&response, created_at).await?;
                Ok(CrawlResultDetail::NotModified { http })
            }
            FeedFetchOutcome::UnexpectedStatus(body) => {
                let (http, body) = self.persist_body(body, created_at).await?;
                Ok(CrawlResultDetail::UnexpectedStatus { http, body })
            }
            FeedFetchOutcome::BodyReadFailed(failure) => {
                let http = self.persist_response(&failure.response, created_at).await?;
                Ok(CrawlResultDetail::BodyReadFailed {
                    http,
                    error: CrawlFetchErrorDetail {
                        kind: failure.failure.kind,
                        message: failure.failure.message,
                    },
                })
            }
            FeedFetchOutcome::FetchFailed(failure) => Ok(CrawlResultDetail::FetchFailed {
                error: CrawlFetchErrorDetail {
                    kind: failure.kind,
                    message: failure.message,
                },
            }),
            FeedFetchOutcome::ParseFailed(failure) => {
                let (http, body) = self.persist_body(failure.body, created_at).await?;
                Ok(CrawlResultDetail::ParseFailed {
                    http,
                    body,
                    error: CrawlFeedParseErrorDetail {
                        kind: failure.failure.kind,
                        message: failure.failure.message,
                    },
                })
            }
        }
    }

    async fn persist_body(
        &mut self,
        body: FeedResponseBody,
        created_at: DateTime<Utc>,
    ) -> RegistryDbResult<(CrawlHttpResponseDetail, CrawlHttpBodyDetail)> {
        let http = self.persist_response(&body.response, created_at).await?;
        let body_blob = self
            .tx
            .put_blob(PutBlobCommand::new(body.bytes, created_at))
            .await?;
        Ok((http, CrawlHttpBodyDetail::new(body_blob)))
    }

    async fn persist_response(
        &mut self,
        response: &FeedHttpResponse,
        created_at: DateTime<Utc>,
    ) -> RegistryDbResult<CrawlHttpResponseDetail> {
        let headers_blob = self
            .tx
            .put_blob(PutBlobCommand::new(
                response.headers.to_json_bytes(),
                created_at,
            ))
            .await?;
        Ok(CrawlHttpResponseDetail {
            status: response.status,
            response_url: response.response_url.clone(),
            headers_blob,
            content_type: response.headers.content_type.clone(),
            content_length: response.headers.content_length,
            etag: response.headers.etag.clone(),
            last_modified: response.headers.last_modified.clone(),
            retry_after_at: retry_after_at(response),
        })
    }
}

/// Current crawl-state facts purely classified from one recorded result
/// detail: the last-result summary and the conditional-fetch headers to use
/// next time.
struct DerivedCrawlState {
    last: LastCrawlResult,
    conditional: FeedConditionalFetch,
}

impl DerivedCrawlState {
    fn derive(
        detail: &CrawlResultDetail,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
        previous_conditional: &FeedConditionalFetch,
    ) -> Self {
        let http = detail.http_response();
        let http_status = http.map(|http| http.status);
        let retry_after = http.and_then(|http| http.retry_after_at);

        let error = match detail {
            CrawlResultDetail::Fetched { .. } | CrawlResultDetail::NotModified { .. } => None,
            CrawlResultDetail::UnexpectedStatus { http, .. } => Some(CrawlStateError::http(
                CrawlHttpErrorKind::from_status(http.status),
            )),
            CrawlResultDetail::BodyReadFailed { error, .. }
            | CrawlResultDetail::FetchFailed { error } => Some(CrawlStateError::fetch(error.kind)),
            CrawlResultDetail::ParseFailed { error, .. } => {
                Some(CrawlStateError::parse(error.kind))
            }
        };
        let last = match error {
            None => LastCrawlResult::normal(started_at, finished_at, http_status, retry_after),
            Some(error) => {
                LastCrawlResult::abnormal(started_at, finished_at, http_status, error, retry_after)
            }
        };

        let conditional = match detail {
            // A parse failure still observed a complete response, so its
            // validators stay usable for the next conditional fetch.
            CrawlResultDetail::Fetched { http, .. }
            | CrawlResultDetail::ParseFailed { http, .. } => FeedConditionalFetch {
                etag: http.etag.clone(),
                last_modified: http.last_modified.clone(),
            },
            CrawlResultDetail::NotModified { http } => FeedConditionalFetch {
                etag: http
                    .etag
                    .clone()
                    .or_else(|| previous_conditional.etag.clone()),
                last_modified: http
                    .last_modified
                    .clone()
                    .or_else(|| previous_conditional.last_modified.clone()),
            },
            CrawlResultDetail::UnexpectedStatus { .. }
            | CrawlResultDetail::BodyReadFailed { .. }
            | CrawlResultDetail::FetchFailed { .. } => previous_conditional.clone(),
        };

        Self { last, conditional }
    }
}

/// Result of recording one crawl completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrawlCompletionRecord {
    pub result_ref: CrawlResultRef,
    pub outcome: CrawlCompletionOutcome,
    pub http_status: Option<FeedHttpStatus>,
    pub error_kind: Option<&'static str>,
    pub health: CrawlHealth,
}

/// Operational summary of one recorded crawl completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CrawlCompletionSummary {
    outcome: CrawlCompletionOutcome,
    http_status: Option<FeedHttpStatus>,
    error_kind: Option<&'static str>,
}

impl CrawlCompletionSummary {
    fn from_detail(detail: &CrawlResultDetail) -> Self {
        let outcome = CrawlCompletionOutcome::from_detail(detail);
        let http_status = detail.http_response().map(|http| http.status);
        let error_kind = match detail {
            CrawlResultDetail::Fetched { .. } | CrawlResultDetail::NotModified { .. } => None,
            CrawlResultDetail::UnexpectedStatus { http, .. } => {
                Some(CrawlHttpErrorKind::from_status(http.status).as_str())
            }
            CrawlResultDetail::BodyReadFailed { error, .. }
            | CrawlResultDetail::FetchFailed { error } => Some(error.kind.as_str()),
            CrawlResultDetail::ParseFailed { error, .. } => Some(error.kind.as_str()),
        };
        Self {
            outcome,
            http_status,
            error_kind,
        }
    }
}

/// Coarse crawl outcome name used for operational logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrawlCompletionOutcome {
    Fetched,
    NotModified,
    UnexpectedStatus,
    BodyReadFailed,
    FetchFailed,
    ParseFailed,
}

impl CrawlCompletionOutcome {
    fn from_detail(detail: &CrawlResultDetail) -> Self {
        match detail {
            CrawlResultDetail::Fetched { .. } => Self::Fetched,
            CrawlResultDetail::NotModified { .. } => Self::NotModified,
            CrawlResultDetail::UnexpectedStatus { .. } => Self::UnexpectedStatus,
            CrawlResultDetail::BodyReadFailed { .. } => Self::BodyReadFailed,
            CrawlResultDetail::FetchFailed { .. } => Self::FetchFailed,
            CrawlResultDetail::ParseFailed { .. } => Self::ParseFailed,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fetched => "fetched",
            Self::NotModified => "not_modified",
            Self::UnexpectedStatus => "unexpected_status",
            Self::BodyReadFailed => "body_read_failed",
            Self::FetchFailed => "fetch_failed",
            Self::ParseFailed => "parse_failed",
        }
    }
}

fn retry_after_at(response: &FeedHttpResponse) -> Option<DateTime<Utc>> {
    response
        .headers
        .retry_after
        .as_deref()
        .and_then(|value| parse_retry_after(value, response.fetched_at))
}

fn parse_retry_after(value: &str, base: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<i64>() {
        return base.checked_add_signed(Duration::seconds(seconds));
    }

    DateTime::parse_from_rfc2822(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}
