use serde::{Deserialize, Deserializer, Serialize};
use synd_feed::types::{Category, FeedType, FeedUrl, Requirement, Time};

use super::PageInfo;

#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionPayload {
    pub feeds: FeedConnection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedConnection {
    pub nodes: Vec<SubscribedFeed>,
    pub page_info: PageInfo,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribedFeed {
    pub url: FeedUrl,
    #[serde(default, with = "super::requirement")]
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub crawl_policy: CrawlPolicy,
    pub feed: Option<FeedDetails>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub struct CrawlPolicy {
    pub polling: PollingPolicy,
}

#[derive(Debug, Clone)]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub enum PollingPolicy {
    Manual,
    Interval {
        seconds: PollingIntervalSeconds,
    },
    Other {
        kind: String,
        interval_seconds: Option<i64>,
    },
}

impl<'de> Deserialize<'de> for PollingPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PollingPolicyWire::deserialize(deserializer)?;
        match (wire.kind.as_str(), wire.interval_seconds) {
            ("MANUAL", None) => Ok(Self::Manual),
            ("MANUAL", Some(_)) => Err(serde::de::Error::custom(
                "MANUAL polling policy must omit intervalSeconds",
            )),
            ("INTERVAL", Some(seconds)) => Ok(Self::Interval {
                seconds: seconds.try_into().map_err(serde::de::Error::custom)?,
            }),
            ("INTERVAL", None) => Err(serde::de::Error::custom(
                "INTERVAL polling policy requires intervalSeconds",
            )),
            (_, interval_seconds) => Ok(Self::Other {
                kind: wire.kind,
                interval_seconds,
            }),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PollingPolicyWire {
    kind: String,
    interval_seconds: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedDetails {
    #[serde(rename = "type")]
    pub feed_type: GraphqlFeedType,
    pub title: Option<String>,
    pub updated: Option<Time>,
    pub website_url: Option<String>,
    pub description: Option<String>,
    pub generator: Option<String>,
    pub entries: EntryMetaConnection,
    pub links: LinkConnection,
    pub authors: AuthorsConnection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphqlFeedType {
    Atom,
    Json,
    Rss0,
    Rss1,
    Rss2,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unsupported GraphQL feed type: {0}")]
pub struct UnsupportedFeedType(pub String);

impl TryFrom<GraphqlFeedType> for FeedType {
    type Error = UnsupportedFeedType;

    fn try_from(value: GraphqlFeedType) -> Result<Self, Self::Error> {
        match value {
            GraphqlFeedType::Atom => Ok(Self::Atom),
            GraphqlFeedType::Json => Ok(Self::JSON),
            GraphqlFeedType::Rss0 => Ok(Self::RSS0),
            GraphqlFeedType::Rss1 => Ok(Self::RSS1),
            GraphqlFeedType::Rss2 => Ok(Self::RSS2),
            GraphqlFeedType::Other(value) => Err(UnsupportedFeedType(value)),
        }
    }
}

impl<'de> Deserialize<'de> for GraphqlFeedType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "ATOM" => Self::Atom,
            "JSON" => Self::Json,
            "RSS0" => Self::Rss0,
            "RSS1" => Self::Rss1,
            "RSS2" => Self::Rss2,
            _ => Self::Other(value),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub struct EntryMetaConnection {
    pub nodes: Vec<EntryMeta>,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub struct LinkConnection {
    pub nodes: Vec<Link>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorsConnection {
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub struct Link {
    pub href: String,
    pub rel: Option<String>,
    pub media_type: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub struct EntryMeta {
    pub title: Option<String>,
    pub published: Option<Time>,
    pub updated: Option<Time>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeFeedInput {
    pub url: FeedUrl,
    #[serde(default, with = "super::requirement")]
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub crawl_policy: Option<CrawlPolicyInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlPolicyInput {
    pub polling: PollingPolicyInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollingPolicyInput {
    Manual,
    Interval { seconds: PollingIntervalSeconds },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub struct PollingIntervalSeconds(i64);

impl PollingIntervalSeconds {
    pub fn get(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("polling interval must be greater than zero")]
pub struct InvalidPollingInterval;

impl TryFrom<i64> for PollingIntervalSeconds {
    type Error = InvalidPollingInterval;

    fn try_from(seconds: i64) -> Result<Self, Self::Error> {
        if seconds > 0 {
            Ok(Self(seconds))
        } else {
            Err(InvalidPollingInterval)
        }
    }
}

impl Serialize for PollingPolicyInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let wire = match self {
            Self::Manual => PollingPolicyInputWire {
                kind: PollingPolicyInputKind::Manual,
                interval_seconds: None,
            },
            Self::Interval { seconds } => PollingPolicyInputWire {
                kind: PollingPolicyInputKind::Interval,
                interval_seconds: Some(seconds.get()),
            },
        };
        wire.serialize(serializer)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PollingPolicyInputWire {
    kind: PollingPolicyInputKind,
    interval_seconds: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum PollingPolicyInputKind {
    Manual,
    Interval,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeFeedPayload {
    pub status: ResponseStatus,
    pub url: FeedUrl,
    pub disposition: SubscribeDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscribeDisposition {
    Subscribed,
    Changed,
    Other(String),
}

impl<'de> Deserialize<'de> for SubscribeDisposition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "SUBSCRIBED" => Self::Subscribed,
            "CHANGED" => Self::Changed,
            _ => Self::Other(value),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeFeedPayload {
    pub status: ResponseStatus,
    pub url: FeedUrl,
    pub disposition: UnsubscribeDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsubscribeDisposition {
    Unsubscribed,
    Other(String),
}

impl<'de> Deserialize<'de> for UnsubscribeDisposition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "UNSUBSCRIBED" => Self::Unsubscribed,
            _ => Self::Other(value),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseStatus {
    pub code: ResponseCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseCode {
    Ok,
    Unauthorized,
    InvalidFeedUrl,
    FeedUnavailable,
    InternalError,
    Other(String),
}

impl<'de> Deserialize<'de> for ResponseCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "OK" => Self::Ok,
            "UNAUTHORIZED" => Self::Unauthorized,
            "INVALID_FEED_URL" => Self::InvalidFeedUrl,
            "FEED_UNAVAILABLE" => Self::FeedUnavailable,
            "INTERNAL_ERROR" => Self::InternalError,
            _ => Self::Other(value),
        })
    }
}
