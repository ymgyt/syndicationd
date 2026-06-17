use serde::{Deserialize, Deserializer, Serialize};
use synd_feed::types::{Category, FeedType, FeedUrl, Requirement, Time};

#[derive(Debug, Clone)]
pub struct FetchEntriesPayload {
    pub entries: Vec<Entry>,
    pub page_info: PageInfo,
}

impl From<EntriesOutput> for FetchEntriesPayload {
    fn from(v: EntriesOutput) -> Self {
        let page_info = v.entries.page_info;
        let entries = v.entries.nodes;

        Self { entries, page_info }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct InitialFeedViewPayload {
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

impl InitialFeedViewPayload {
    pub fn into_parts(self) -> (Option<SubscriptionPayload>, Option<FetchEntriesPayload>) {
        let subscriptions = match self.subscriptions {
            InitialSubscriptionsPayload::Ready(feeds) => Some(SubscriptionPayload { feeds }),
            InitialSubscriptionsPayload::Unavailable => None,
        };
        let timeline = match self.timeline {
            InitialTimelinePayload::Ready(timeline) => Some(FetchEntriesPayload {
                entries: timeline.entries.nodes,
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
    pub page_info: PageInfo,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub title: Option<String>,
    pub published: Option<Time>,
    pub updated: Option<Time>,
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
    pub page_info: PageInfo,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub struct PageInfo {
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribedFeed {
    pub url: FeedUrl,
    #[serde(default, with = "requirement_graphql")]
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub crawl_policy: CrawlPolicy,
    pub refresh_status: Option<RefreshStatus>,
    pub feed: Option<FeedDetails>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub struct CrawlPolicy {
    pub polling: PollingPolicy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub struct PollingPolicy {
    pub kind: PollingPolicyKind,
    pub interval_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub enum PollingPolicyKind {
    Manual,
    Interval,
    Other(String),
}

impl<'de> Deserialize<'de> for PollingPolicyKind {
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
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub struct RefreshStatus {
    pub state: RefreshStatusState,
    pub request_id: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
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

impl RefreshStatus {
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }
}

impl From<&RefreshDisposition> for RefreshStatusState {
    fn from(value: &RefreshDisposition) -> Self {
        match value {
            RefreshDisposition::JoinedRunning => Self::Running,
            RefreshDisposition::Created
            | RefreshDisposition::Promoted
            | RefreshDisposition::CoalescedPending
            | RefreshDisposition::Other(_) => Self::Pending,
        }
    }
}

impl From<&RefreshFeedPayload> for RefreshStatus {
    fn from(value: &RefreshFeedPayload) -> Self {
        Self {
            state: RefreshStatusState::from(&value.disposition),
            request_id: Some(value.request_id.clone()),
            last_attempt_at: None,
            last_success_at: None,
            last_failure_at: None,
            last_error_message: None,
        }
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
#[serde(tag = "__typename")]
pub enum FeedEvent {
    TimelineChanged(TimelineChangeEvent),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineChangeEvent {
    pub changed_at: String,
    pub affected_feeds: Option<Vec<FeedUrl>>,
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
    #[serde(default, with = "requirement_graphql")]
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub crawl_policy: Option<CrawlPolicyInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlPolicyInput {
    pub polling: PollingPolicyInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollingPolicyInput {
    pub kind: PollingPolicyInputKind,
    pub interval_seconds: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PollingPolicyInputKind {
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

#[cfg(test)]
mod tests {
    use core::assert_matches;

    use super::FeedEvent;

    #[test]
    fn decodes_timeline_changed_feed_event() {
        let event: FeedEvent = serde_json::from_value(serde_json::json!({
            "__typename": "TimelineChanged",
            "changedAt": "2026-06-13T00:00:00Z",
            "affectedFeeds": ["https://example.com/feed.xml"]
        }))
        .unwrap();

        let FeedEvent::TimelineChanged(event) = event;
        assert_eq!(event.changed_at, "2026-06-13T00:00:00Z");
        assert_matches!(event.affected_feeds, Some(feeds) if feeds.len() == 1);
    }
}
