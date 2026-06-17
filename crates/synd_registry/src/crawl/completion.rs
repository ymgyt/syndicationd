use chrono::{DateTime, Duration, Utc};
use synd_feed::feed::service::{
    FeedConditionalFetch, FeedFetchOutcome, FeedHttpResponse, FeedResponseBody,
};

use crate::{
    crawl::{
        blob::PutBlobCommand,
        job::{CrawlJob, FinishCrawlJobCommand, FinishCrawlJobOutcome},
        result::{
            CrawlFeedParseErrorDetail, CrawlFetchErrorDetail, CrawlHealth, CrawlHttpBodyDetail,
            CrawlHttpErrorKind, CrawlHttpResponseDetail, CrawlResultDetail, CrawlResultRecord,
            CrawlResultRef, CrawlState, CrawlStateError, LastCrawlResult, RecordCrawlResultCommand,
            UpsertCrawlStateCommand,
        },
    },
    db::{BlobStore, CrawlJobQueue, CrawlResultStore},
    error::{RegistryDbError, RegistryDbResult},
    event::{CrawlJobFinishedEvent, Event},
};

/// Records the durable completion facts for one claimed crawl job.
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
    Tx: BlobStore + CrawlResultStore + CrawlJobQueue + Send,
{
    pub async fn record(
        &mut self,
        job: CrawlJob,
        outcome: FeedFetchOutcome,
        previous_state: Option<CrawlState>,
        finished_at: DateTime<Utc>,
    ) -> RegistryDbResult<(CrawlCompletionRecord, Vec<Event>)> {
        let started_at = job.updated_at;
        let feed_url = job.feed_url.clone();
        let previous_conditional = previous_state
            .as_ref()
            .map(|state| state.conditional.clone())
            .unwrap_or_default();

        let observed = self
            .persist_outcome(outcome, started_at, finished_at, &previous_conditional)
            .await?;

        let result_ref = self
            .tx
            .record_crawl_result(RecordCrawlResultCommand::new(
                CrawlResultRecord::new(
                    job.job_id.clone(),
                    feed_url.clone(),
                    started_at,
                    finished_at,
                ),
                observed.detail,
            ))
            .await?;

        let health = CrawlHealth::for_last_result(&observed.last, previous_state.as_ref());
        self.tx
            .upsert_crawl_state(UpsertCrawlStateCommand::new(
                result_ref,
                feed_url,
                observed.last,
                health,
                observed.conditional,
                finished_at,
            ))
            .await?;

        let finished_job = match self
            .tx
            .finish_job(FinishCrawlJobCommand::new(job.job_id, finished_at))
            .await?
        {
            FinishCrawlJobOutcome::Finished(job) => job,
            FinishCrawlJobOutcome::NotRunning => {
                return Err(RegistryDbError::internal_message(
                    "claimed crawl job was not running when completion was recorded",
                ));
            }
        };

        Ok((
            CrawlCompletionRecord { result_ref },
            vec![CrawlJobFinishedEvent::from(finished_job).into()],
        ))
    }

    async fn persist_outcome(
        &mut self,
        outcome: FeedFetchOutcome,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
        previous_conditional: &FeedConditionalFetch,
    ) -> RegistryDbResult<PersistedOutcome> {
        match outcome {
            FeedFetchOutcome::Fetched(fetched) => {
                let fetched = *fetched;
                let response = fetched.body.response.clone();
                let (http, body) = self.persist_body(fetched.body, finished_at).await?;
                Ok(PersistedOutcome {
                    detail: CrawlResultDetail::Fetched { http, body },
                    last: LastCrawlResult::normal(
                        started_at,
                        finished_at,
                        Some(response.status),
                        retry_after_at(&response),
                    ),
                    conditional: conditional_from_success(&response),
                })
            }
            FeedFetchOutcome::NotModified(response) => {
                let http = self.persist_response(&response, finished_at).await?;
                Ok(PersistedOutcome {
                    detail: CrawlResultDetail::NotModified { http },
                    last: LastCrawlResult::normal(
                        started_at,
                        finished_at,
                        Some(response.status),
                        retry_after_at(&response),
                    ),
                    conditional: conditional_from_not_modified(&response, previous_conditional),
                })
            }
            FeedFetchOutcome::UnexpectedStatus(body) => {
                let response = body.response.clone();
                let (http, body) = self.persist_body(body, finished_at).await?;
                let error = CrawlStateError::http(CrawlHttpErrorKind::from_status(response.status));
                Ok(PersistedOutcome {
                    detail: CrawlResultDetail::UnexpectedStatus { http, body },
                    last: LastCrawlResult::abnormal(
                        started_at,
                        finished_at,
                        Some(response.status),
                        error,
                        retry_after_at(&response),
                    ),
                    conditional: previous_conditional.clone(),
                })
            }
            FeedFetchOutcome::BodyReadFailed(failure) => {
                let http = self
                    .persist_response(&failure.response, finished_at)
                    .await?;
                let error = CrawlFetchErrorDetail {
                    kind: failure.failure.kind,
                    message: failure.failure.message,
                };
                Ok(PersistedOutcome {
                    detail: CrawlResultDetail::BodyReadFailed {
                        http,
                        error: error.clone(),
                    },
                    last: LastCrawlResult::abnormal(
                        started_at,
                        finished_at,
                        Some(failure.response.status),
                        CrawlStateError::fetch(error.kind),
                        retry_after_at(&failure.response),
                    ),
                    conditional: previous_conditional.clone(),
                })
            }
            FeedFetchOutcome::FetchFailed(failure) => {
                let error = CrawlFetchErrorDetail {
                    kind: failure.kind,
                    message: failure.message,
                };
                Ok(PersistedOutcome {
                    detail: CrawlResultDetail::FetchFailed {
                        error: error.clone(),
                    },
                    last: LastCrawlResult::abnormal(
                        started_at,
                        finished_at,
                        None,
                        CrawlStateError::fetch(error.kind),
                        None,
                    ),
                    conditional: previous_conditional.clone(),
                })
            }
            FeedFetchOutcome::ParseFailed(failure) => {
                let response = failure.body.response.clone();
                let (http, body) = self.persist_body(failure.body, finished_at).await?;
                let error = CrawlFeedParseErrorDetail {
                    kind: failure.failure.kind,
                    message: failure.failure.message,
                };
                Ok(PersistedOutcome {
                    detail: CrawlResultDetail::ParseFailed {
                        http,
                        body,
                        error: error.clone(),
                    },
                    last: LastCrawlResult::abnormal(
                        started_at,
                        finished_at,
                        Some(response.status),
                        CrawlStateError::parse(error.kind),
                        retry_after_at(&response),
                    ),
                    conditional: conditional_from_success(&response),
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

/// Result of recording one crawl completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrawlCompletionRecord {
    pub result_ref: CrawlResultRef,
}

struct PersistedOutcome {
    detail: CrawlResultDetail,
    last: LastCrawlResult,
    conditional: FeedConditionalFetch,
}

fn conditional_from_success(response: &FeedHttpResponse) -> FeedConditionalFetch {
    FeedConditionalFetch {
        etag: response.headers.etag.clone(),
        last_modified: response.headers.last_modified.clone(),
    }
}

fn conditional_from_not_modified(
    response: &FeedHttpResponse,
    previous: &FeedConditionalFetch,
) -> FeedConditionalFetch {
    FeedConditionalFetch {
        etag: response
            .headers
            .etag
            .clone()
            .or_else(|| previous.etag.clone()),
        last_modified: response
            .headers
            .last_modified
            .clone()
            .or_else(|| previous.last_modified.clone()),
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
