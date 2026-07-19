use synd_feed::feed::service::{FeedFetchFailureKind, FeedParseErrorKind};
use synd_registry::{
    crawl::{
        policy::CrawlPolicy,
        state::{CrawlHttpErrorKind, CrawlStateErrorKind},
    },
    entry::EntryAttrs,
};

use super::error::{SqliteError, SqliteResult};

pub(super) fn encode_crawl_policy_json(policy: CrawlPolicy) -> SqliteResult<String> {
    Ok(serde_json::to_string(&policy)?)
}

pub(super) fn decode_crawl_policy_json(policy_json: &str) -> SqliteResult<CrawlPolicy> {
    Ok(serde_json::from_str(policy_json)?)
}

pub(super) fn encode_entry_attrs_json(attrs: &EntryAttrs) -> SqliteResult<String> {
    Ok(serde_json::to_string(attrs)?)
}

pub(super) fn decode_entry_attrs_json(attrs_json: &str) -> SqliteResult<EntryAttrs> {
    Ok(serde_json::from_str(attrs_json)?)
}

pub(super) fn encode_crawl_state_error_kind(kind: CrawlStateErrorKind) -> String {
    match kind {
        CrawlStateErrorKind::Fetch(kind) => format!("fetch_{}", kind.as_str()),
        CrawlStateErrorKind::Http(kind) => format!("http_{}", kind.as_str()),
        CrawlStateErrorKind::Parse(kind) => format!("parse_{}", kind.as_str()),
    }
}

pub(super) fn decode_crawl_state_error_kind(value: &str) -> SqliteResult<CrawlStateErrorKind> {
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

pub(super) fn decode_fetch_failure_kind(value: &str) -> SqliteResult<FeedFetchFailureKind> {
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

pub(super) fn decode_parse_error_kind(value: &str) -> SqliteResult<FeedParseErrorKind> {
    match value {
        "invalid_feed" => Ok(FeedParseErrorKind::InvalidFeed),
        "io" => Ok(FeedParseErrorKind::Io),
        "json_format" => Ok(FeedParseErrorKind::JsonFormat),
        "json_unsupported_version" => Ok(FeedParseErrorKind::JsonUnsupportedVersion),
        "xml_format" => Ok(FeedParseErrorKind::XmlFormat),
        value => Err(unknown_value("feed parse error kind", value)),
    }
}

fn decode_http_error_kind(value: &str) -> SqliteResult<CrawlHttpErrorKind> {
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

fn unknown_value(field: &'static str, value: &str) -> SqliteError {
    SqliteError::decode_message(format!("unknown {field}: {value}"))
}

#[cfg(test)]
mod tests {
    use synd_feed::feed::service::{FeedFetchFailureKind, FeedParseErrorKind};
    use synd_registry::crawl::state::{CrawlHttpErrorKind, CrawlStateErrorKind};

    use super::{decode_crawl_state_error_kind, encode_crawl_state_error_kind};

    #[test]
    fn crawl_state_error_kind_round_trips() {
        let cases = [
            CrawlStateErrorKind::Fetch(FeedFetchFailureKind::Connect),
            CrawlStateErrorKind::Fetch(FeedFetchFailureKind::Timeout),
            CrawlStateErrorKind::Fetch(FeedFetchFailureKind::Request),
            CrawlStateErrorKind::Fetch(FeedFetchFailureKind::Body),
            CrawlStateErrorKind::Fetch(FeedFetchFailureKind::TooLarge),
            CrawlStateErrorKind::Fetch(FeedFetchFailureKind::Unsupported),
            CrawlStateErrorKind::Fetch(FeedFetchFailureKind::Other),
            CrawlStateErrorKind::Http(CrawlHttpErrorKind::RateLimited),
            CrawlStateErrorKind::Http(CrawlHttpErrorKind::Unavailable),
            CrawlStateErrorKind::Http(CrawlHttpErrorKind::NotFound),
            CrawlStateErrorKind::Http(CrawlHttpErrorKind::Gone),
            CrawlStateErrorKind::Http(CrawlHttpErrorKind::ClientError),
            CrawlStateErrorKind::Http(CrawlHttpErrorKind::ServerError),
            CrawlStateErrorKind::Http(CrawlHttpErrorKind::UnexpectedStatus),
            CrawlStateErrorKind::Parse(FeedParseErrorKind::InvalidFeed),
            CrawlStateErrorKind::Parse(FeedParseErrorKind::Io),
            CrawlStateErrorKind::Parse(FeedParseErrorKind::JsonFormat),
            CrawlStateErrorKind::Parse(FeedParseErrorKind::JsonUnsupportedVersion),
            CrawlStateErrorKind::Parse(FeedParseErrorKind::XmlFormat),
        ];

        for case in cases {
            let encoded = encode_crawl_state_error_kind(case);
            assert_eq!(decode_crawl_state_error_kind(&encoded).unwrap(), case);
        }
    }
}
