use chrono::{DateTime, Duration, Utc};
use synd_feed::feed::service::{
    FeedConditionalFetch, FeedFetchOutcome, FeedHttpResponse, FeedHttpStatus,
};

use crate::crawl::state::{CrawlHttpErrorKind, CrawlStateError, LastCrawlResult};

/// Pure classification of one fetch outcome into the facts a finished crawl
/// leaves behind: the last-result summary, the conditional-fetch headers to
/// use next time, and the accepted body to keep, if any.
pub(crate) struct CrawlCompletion {
    pub(crate) last: LastCrawlResult,
    pub(crate) conditional: FeedConditionalFetch,
    /// Body bytes of a successfully fetched and parsed feed. Failure bodies
    /// are not kept: they have no reader.
    pub(crate) body: Option<Vec<u8>>,
    pub(crate) summary: CrawlCompletionSummary,
}

impl CrawlCompletion {
    pub(crate) fn classify(
        outcome: FeedFetchOutcome,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
        previous_conditional: &FeedConditionalFetch,
    ) -> Self {
        let summary = CrawlCompletionSummary::from_outcome(&outcome);
        let http = outcome_response(&outcome);
        let http_status = http.map(|http| http.status);
        let retry_after = http.and_then(retry_after_at);

        let error = match &outcome {
            FeedFetchOutcome::Fetched(_) | FeedFetchOutcome::NotModified(_) => None,
            FeedFetchOutcome::UnexpectedStatus(body) => Some(CrawlStateError::http(
                CrawlHttpErrorKind::from_status(body.response.status),
            )),
            FeedFetchOutcome::BodyReadFailed(failure) => {
                Some(CrawlStateError::fetch(failure.failure.kind))
            }
            FeedFetchOutcome::FetchFailed(failure) => Some(CrawlStateError::fetch(failure.kind)),
            FeedFetchOutcome::ParseFailed(failure) => {
                Some(CrawlStateError::parse(failure.failure.kind))
            }
        };
        let last = match error {
            None => LastCrawlResult::normal(started_at, finished_at, http_status, retry_after),
            Some(error) => {
                LastCrawlResult::abnormal(started_at, finished_at, http_status, error, retry_after)
            }
        };

        let conditional = match &outcome {
            // A parse failure still observed a complete response, so its
            // validators stay usable for the next conditional fetch.
            FeedFetchOutcome::Fetched(fetched) => validators(&fetched.body.response),
            FeedFetchOutcome::ParseFailed(failure) => validators(&failure.body.response),
            FeedFetchOutcome::NotModified(response) => FeedConditionalFetch {
                etag: response
                    .headers
                    .etag
                    .clone()
                    .or_else(|| previous_conditional.etag.clone()),
                last_modified: response
                    .headers
                    .last_modified
                    .clone()
                    .or_else(|| previous_conditional.last_modified.clone()),
            },
            FeedFetchOutcome::UnexpectedStatus(_)
            | FeedFetchOutcome::BodyReadFailed(_)
            | FeedFetchOutcome::FetchFailed(_) => previous_conditional.clone(),
        };

        let body = match outcome {
            FeedFetchOutcome::Fetched(fetched) => Some(fetched.body.bytes),
            _ => None,
        };

        Self {
            last,
            conditional,
            body,
            summary,
        }
    }
}

/// Operational summary of one crawl completion, used for logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CrawlCompletionSummary {
    pub(crate) outcome: CrawlCompletionOutcome,
    pub(crate) http_status: Option<FeedHttpStatus>,
    pub(crate) error_kind: Option<&'static str>,
}

impl CrawlCompletionSummary {
    fn from_outcome(outcome: &FeedFetchOutcome) -> Self {
        let http_status = outcome_response(outcome).map(|http| http.status);
        let (name, error_kind) = match outcome {
            FeedFetchOutcome::Fetched(_) => (CrawlCompletionOutcome::Fetched, None),
            FeedFetchOutcome::NotModified(_) => (CrawlCompletionOutcome::NotModified, None),
            FeedFetchOutcome::UnexpectedStatus(body) => (
                CrawlCompletionOutcome::UnexpectedStatus,
                Some(CrawlHttpErrorKind::from_status(body.response.status).as_str()),
            ),
            FeedFetchOutcome::BodyReadFailed(failure) => (
                CrawlCompletionOutcome::BodyReadFailed,
                Some(failure.failure.kind.as_str()),
            ),
            FeedFetchOutcome::FetchFailed(failure) => (
                CrawlCompletionOutcome::FetchFailed,
                Some(failure.kind.as_str()),
            ),
            FeedFetchOutcome::ParseFailed(failure) => (
                CrawlCompletionOutcome::ParseFailed,
                Some(failure.failure.kind.as_str()),
            ),
        };
        Self {
            outcome: name,
            http_status,
            error_kind,
        }
    }
}

/// Coarse crawl outcome name used for operational logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CrawlCompletionOutcome {
    Fetched,
    NotModified,
    UnexpectedStatus,
    BodyReadFailed,
    FetchFailed,
    ParseFailed,
}

impl CrawlCompletionOutcome {
    pub(crate) fn as_str(self) -> &'static str {
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

fn outcome_response(outcome: &FeedFetchOutcome) -> Option<&FeedHttpResponse> {
    match outcome {
        FeedFetchOutcome::Fetched(fetched) => Some(&fetched.body.response),
        FeedFetchOutcome::NotModified(response) => Some(response),
        FeedFetchOutcome::UnexpectedStatus(body) => Some(&body.response),
        FeedFetchOutcome::BodyReadFailed(failure) => Some(&failure.response),
        FeedFetchOutcome::ParseFailed(failure) => Some(&failure.body.response),
        FeedFetchOutcome::FetchFailed(_) => None,
    }
}

fn validators(response: &FeedHttpResponse) -> FeedConditionalFetch {
    FeedConditionalFetch {
        etag: response.headers.etag.clone(),
        last_modified: response.headers.last_modified.clone(),
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
