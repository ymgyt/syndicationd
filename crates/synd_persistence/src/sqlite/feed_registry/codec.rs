use synd_feed::feed::service::{FeedFetchFailureKind, FeedParseErrorKind};
use synd_registry::{
    RegistryDbError, RegistryDbResult,
    crawl::{
        policy::CrawlPolicy,
        result::{CrawlHttpErrorKind, CrawlStateErrorKind},
    },
};

pub(super) fn encode_crawl_policy_json(policy: CrawlPolicy) -> RegistryDbResult<String> {
    serde_json::to_string(&policy).map_err(RegistryDbError::internal)
}

pub(super) fn decode_crawl_policy_json(policy_json: &str) -> RegistryDbResult<CrawlPolicy> {
    serde_json::from_str(policy_json).map_err(RegistryDbError::internal)
}

pub(super) fn encode_crawl_state_error_kind(kind: CrawlStateErrorKind) -> String {
    match kind {
        CrawlStateErrorKind::Fetch(kind) => format!("fetch_{}", kind.as_str()),
        CrawlStateErrorKind::Http(kind) => format!("http_{}", kind.as_str()),
        CrawlStateErrorKind::Parse(kind) => format!("parse_{}", kind.as_str()),
    }
}

pub(super) fn decode_crawl_state_error_kind(value: &str) -> RegistryDbResult<CrawlStateErrorKind> {
    if let Some(kind) = value.strip_prefix("fetch_") {
        return decode_fetch_failure_kind(kind).map(CrawlStateErrorKind::Fetch);
    }
    if let Some(kind) = value.strip_prefix("http_") {
        return decode_http_error_kind(kind).map(CrawlStateErrorKind::Http);
    }
    if let Some(kind) = value.strip_prefix("parse_") {
        return decode_parse_error_kind(kind).map(CrawlStateErrorKind::Parse);
    }
    Err(unknown_value("crawl state error kind", value))
}

pub(super) fn decode_fetch_failure_kind(value: &str) -> RegistryDbResult<FeedFetchFailureKind> {
    match value {
        "connect" => Ok(FeedFetchFailureKind::Connect),
        "timeout" => Ok(FeedFetchFailureKind::Timeout),
        "request" => Ok(FeedFetchFailureKind::Request),
        "body" => Ok(FeedFetchFailureKind::Body),
        "too_large" => Ok(FeedFetchFailureKind::TooLarge),
        "unsupported" => Ok(FeedFetchFailureKind::Unsupported),
        "other" => Ok(FeedFetchFailureKind::Other),
        value => Err(unknown_value("feed fetch failure kind", value)),
    }
}

pub(super) fn decode_parse_error_kind(value: &str) -> RegistryDbResult<FeedParseErrorKind> {
    match value {
        "invalid_feed" => Ok(FeedParseErrorKind::InvalidFeed),
        "io" => Ok(FeedParseErrorKind::Io),
        "json_format" => Ok(FeedParseErrorKind::JsonFormat),
        "json_unsupported_version" => Ok(FeedParseErrorKind::JsonUnsupportedVersion),
        "xml_format" => Ok(FeedParseErrorKind::XmlFormat),
        value => Err(unknown_value("feed parse error kind", value)),
    }
}

fn decode_http_error_kind(value: &str) -> RegistryDbResult<CrawlHttpErrorKind> {
    match value {
        "rate_limited" => Ok(CrawlHttpErrorKind::RateLimited),
        "unavailable" => Ok(CrawlHttpErrorKind::Unavailable),
        "not_found" => Ok(CrawlHttpErrorKind::NotFound),
        "gone" => Ok(CrawlHttpErrorKind::Gone),
        "client_error" => Ok(CrawlHttpErrorKind::ClientError),
        "server_error" => Ok(CrawlHttpErrorKind::ServerError),
        "unexpected_status" => Ok(CrawlHttpErrorKind::UnexpectedStatus),
        value => Err(unknown_value("crawl http error kind", value)),
    }
}

fn unknown_value(field: &'static str, value: &str) -> RegistryDbError {
    RegistryDbError::internal(anyhow::anyhow!("unknown {field}: {value}"))
}
