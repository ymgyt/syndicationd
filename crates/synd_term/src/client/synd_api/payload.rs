use serde::{Deserialize, Deserializer, Serialize};
use synd_feed::types::{Category, FeedType, FeedUrl, Requirement};

use crate::types;

#[derive(Debug, Clone)]
pub struct FetchEntriesPayload {
    pub entries: Vec<types::Entry>,
    pub page_info: types::PageInfo,
}

impl From<EntriesOutput> for FetchEntriesPayload {
    fn from(v: EntriesOutput) -> Self {
        let page_info = v.entries.page_info;
        let entries = v.entries.nodes.into_iter().map(Into::into).collect();

        Self { entries, page_info }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct InitialFeedRegistryPayload {
    #[serde(default)]
    pub subscriptions: InitialSubscriptionsPayload,
    #[serde(default)]
    pub timeline: InitialTimelinePayload,
}

#[derive(Debug, Clone, Default)]
pub enum InitialSubscriptionsPayload {
    Ready(FeedConnection),
    #[default]
    Unavailable,
}

impl<'de> Deserialize<'de> for InitialSubscriptionsPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match Option::<FeedConnection>::deserialize(deserializer)? {
            Some(feeds) => Self::Ready(feeds),
            None => Self::Unavailable,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub enum InitialTimelinePayload {
    Ready(TimelinePayload),
    #[default]
    Unavailable,
}

impl<'de> Deserialize<'de> for InitialTimelinePayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(
            match Option::<TimelinePayload>::deserialize(deserializer)? {
                Some(timeline) => Self::Ready(timeline),
                None => Self::Unavailable,
            },
        )
    }
}

impl InitialFeedRegistryPayload {
    pub fn into_parts(self) -> (Option<SubscriptionPayload>, Option<FetchEntriesPayload>) {
        let subscriptions = match self.subscriptions {
            InitialSubscriptionsPayload::Ready(feeds) => Some(SubscriptionPayload { feeds }),
            InitialSubscriptionsPayload::Unavailable => None,
        };
        let timeline = match self.timeline {
            InitialTimelinePayload::Ready(timeline) => Some(FetchEntriesPayload {
                entries: timeline.entries.nodes.into_iter().map(Into::into).collect(),
                page_info: timeline.entries.page_info,
            }),
            InitialTimelinePayload::Unavailable => None,
        };
        (subscriptions, timeline)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimelinePayload {
    pub entries: EntryConnection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntriesResponseData {
    pub output: EntriesOutput,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntriesOutput {
    pub entries: EntryConnection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryConnection {
    pub nodes: Vec<Entry>,
    pub page_info: types::PageInfo,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub title: Option<String>,
    pub published: Option<String>,
    pub updated: Option<String>,
    pub website_url: Option<String>,
    pub summary: Option<String>,
    pub feed: FeedMeta,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedMeta {
    pub title: Option<String>,
    pub url: FeedUrl,
    #[serde(default, with = "requirement_graphql")]
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionPayload {
    pub feeds: FeedConnection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedConnection {
    pub nodes: Vec<SubscribedFeed>,
    pub page_info: types::PageInfo,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribedFeed {
    pub url: FeedUrl,
    #[serde(default, with = "requirement_graphql")]
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub refresh_policy: RefreshPolicy,
    pub refresh_status: RefreshStatus,
    pub feed: Option<FeedDetails>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshPolicy {
    pub kind: RefreshPolicyKind,
    pub interval_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshPolicyKind {
    Manual,
    Interval,
    Other(String),
}

impl<'de> Deserialize<'de> for RefreshPolicyKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "MANUAL" => Self::Manual,
            "INTERVAL" => Self::Interval,
            _ => Self::Other(value),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshStatus {
    pub state: RefreshStatusState,
    pub request_id: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshStatusState {
    NeverRefreshed,
    Idle,
    Pending,
    Running,
    LastFailed,
    Other(String),
}

impl RefreshStatusState {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Pending | Self::Running)
    }
}

impl<'de> Deserialize<'de> for RefreshStatusState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "NEVER_REFRESHED" => Self::NeverRefreshed,
            "IDLE" => Self::Idle,
            "PENDING" => Self::Pending,
            "RUNNING" => Self::Running,
            "LAST_FAILED" => Self::LastFailed,
            _ => Self::Other(value),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedStatusOutput {
    pub feed_status: RefreshStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedStatusResponseData {
    pub output: FeedStatusOutput,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineChangedEvent {
    pub changed_at: String,
    pub affected_feeds: Option<Vec<FeedUrl>>,
}

#[derive(Debug, Clone)]
pub enum FeedRegistryEvent {
    TimelineChanged(TimelineChangedEvent),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedDetails {
    #[serde(rename = "type")]
    pub feed_type: GraphqlFeedType,
    pub title: Option<String>,
    pub updated: Option<String>,
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

impl GraphqlFeedType {
    pub fn into_feed_type(self) -> Option<FeedType> {
        match self {
            Self::Atom => Some(FeedType::Atom),
            Self::Json => Some(FeedType::JSON),
            Self::Rss0 => Some(FeedType::RSS0),
            Self::Rss1 => Some(FeedType::RSS1),
            Self::Rss2 => Some(FeedType::RSS2),
            Self::Other(_) => None,
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
pub struct EntryMetaConnection {
    pub nodes: Vec<types::EntryMeta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LinkConnection {
    pub nodes: Vec<types::Link>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorsConnection {
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeFeedInput {
    pub url: FeedUrl,
    #[serde(default, with = "requirement_graphql")]
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub refresh_policy: Option<RefreshPolicyInput>,
    pub initial_refresh: Option<InitialRefreshModeInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InitialRefreshModeInput {
    Async,
    RequireSuccess,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshPolicyInput {
    pub kind: RefreshPolicyInputKind,
    pub interval_seconds: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefreshPolicyInputKind {
    Manual,
    Interval,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeFeedPayload {
    pub status: ResponseStatus,
    pub url: FeedUrl,
    pub request_id: Option<String>,
    pub disposition: Option<RefreshDisposition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshFeedPayload {
    pub status: ResponseStatus,
    pub request_id: String,
    pub disposition: RefreshDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshDisposition {
    Created,
    Promoted,
    CoalescedPending,
    JoinedRunning,
    Other(String),
}

impl<'de> Deserialize<'de> for RefreshDisposition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "CREATED" => Self::Created,
            "PROMOTED" => Self::Promoted,
            "COALESCED_PENDING" => Self::CoalescedPending,
            "JOINED_RUNNING" => Self::JoinedRunning,
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

#[derive(Debug, Clone)]
pub struct ExportSubscriptionPayload {
    pub feeds: Vec<types::ExportedFeed>,
    pub page_info: types::PageInfo,
}

mod requirement_graphql {
    use serde::{Deserialize, Deserializer, Serializer, de};
    use synd_feed::types::Requirement;

    const VARIANTS: &[&str] = &["MUST", "SHOULD", "MAY"];

    #[allow(clippy::ref_option, clippy::trivially_copy_pass_by_ref)]
    pub(super) fn serialize<S>(
        value: &Option<Requirement>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(Requirement::Must) => serializer.serialize_some("MUST"),
            Some(Requirement::Should) => serializer.serialize_some("SHOULD"),
            Some(Requirement::May) => serializer.serialize_some("MAY"),
            None => serializer.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Option<Requirement>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let Some(value) = Option::<String>::deserialize(deserializer)? else {
            return Ok(None);
        };

        match value.as_str() {
            "MUST" => Ok(Some(Requirement::Must)),
            "SHOULD" => Ok(Some(Requirement::Should)),
            "MAY" => Ok(Some(Requirement::May)),
            value => Err(de::Error::unknown_variant(value, VARIANTS)),
        }
    }
}
