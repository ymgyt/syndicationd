use chrono::DateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synd_feed::types::{Category, FeedType, FeedUrl, Requirement};
use tracing::warn;

use crate::{client::synd_api::payload, ui};

mod time;
pub use time::{Time, TimeExt};

mod page_info;
pub use page_info::PageInfo;

mod requirement_ext;
pub use requirement_ext::RequirementExt;

pub(crate) mod github;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(fake::Dummy))]
pub struct Link {
    pub href: String,
    pub rel: Option<String>,
    pub media_type: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(test, derive(fake::Dummy))]
pub struct EntryMeta {
    pub title: Option<String>,
    pub published: Option<Time>,
    pub updated: Option<Time>,
    pub summary: Option<String>,
}

impl EntryMeta {
    pub fn summary_text(&self, width: usize) -> Option<String> {
        self.summary.as_deref().and_then(|summary| {
            match html2text::from_read(summary.as_bytes(), width) {
                Ok(text) => Some(text),
                Err(err) => {
                    warn!("convert summary html to text: {err}");
                    None
                }
            }
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(fake::Dummy))]
pub struct FeedRefreshPolicy {
    pub kind: FeedRefreshPolicyKind,
    pub interval_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(fake::Dummy))]
pub enum FeedRefreshPolicyKind {
    Manual,
    Interval,
    Other(String),
}

impl From<payload::RefreshPolicy> for FeedRefreshPolicy {
    fn from(value: payload::RefreshPolicy) -> Self {
        Self {
            kind: value.kind.into(),
            interval_seconds: value.interval_seconds,
        }
    }
}

impl FeedRefreshPolicy {
    pub fn prompt_value(&self) -> Option<String> {
        match self.kind {
            FeedRefreshPolicyKind::Manual => Some("manual".to_owned()),
            FeedRefreshPolicyKind::Interval => self
                .interval_seconds
                .filter(|seconds| *seconds > 0)
                .map(|seconds| format!("interval:{seconds}s")),
            FeedRefreshPolicyKind::Other(_) => None,
        }
    }
}

impl From<payload::RefreshPolicyKind> for FeedRefreshPolicyKind {
    fn from(value: payload::RefreshPolicyKind) -> Self {
        match value {
            payload::RefreshPolicyKind::Manual => Self::Manual,
            payload::RefreshPolicyKind::Interval => Self::Interval,
            payload::RefreshPolicyKind::Other(value) => Self::Other(value),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(fake::Dummy))]
pub struct FeedRefreshStatus {
    pub state: FeedRefreshStatusState,
    pub request_id: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_error_message: Option<String>,
}

impl FeedRefreshStatus {
    pub fn from_refresh_receipt(value: &payload::RefreshFeedPayload) -> Self {
        Self {
            state: FeedRefreshStatusState::from(&value.disposition),
            request_id: Some(value.request_id.clone()),
            last_attempt_at: None,
            last_success_at: None,
            last_failure_at: None,
            last_error_message: None,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            FeedRefreshStatusState::Pending | FeedRefreshStatusState::Running
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(fake::Dummy))]
pub enum FeedRefreshStatusState {
    NeverRefreshed,
    Idle,
    Pending,
    Running,
    LastFailed,
    Other(String),
}

impl FeedRefreshStatusState {
    pub fn label(&self) -> &str {
        match self {
            Self::NeverRefreshed => "Never",
            Self::Idle => "Idle",
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::LastFailed => "Failed",
            Self::Other(_) => "Unknown",
        }
    }
}

impl From<payload::RefreshStatus> for FeedRefreshStatus {
    fn from(value: payload::RefreshStatus) -> Self {
        Self {
            state: value.state.into(),
            request_id: value.request_id,
            last_attempt_at: value.last_attempt_at,
            last_success_at: value.last_success_at,
            last_failure_at: value.last_failure_at,
            last_error_message: value.last_error_message,
        }
    }
}

impl From<payload::RefreshStatusState> for FeedRefreshStatusState {
    fn from(value: payload::RefreshStatusState) -> Self {
        match value {
            payload::RefreshStatusState::NeverRefreshed => Self::NeverRefreshed,
            payload::RefreshStatusState::Idle => Self::Idle,
            payload::RefreshStatusState::Pending => Self::Pending,
            payload::RefreshStatusState::Running => Self::Running,
            payload::RefreshStatusState::LastFailed => Self::LastFailed,
            payload::RefreshStatusState::Other(value) => Self::Other(value),
        }
    }
}

impl From<&payload::RefreshDisposition> for FeedRefreshStatusState {
    fn from(value: &payload::RefreshDisposition) -> Self {
        match value {
            payload::RefreshDisposition::JoinedRunning => Self::Running,
            payload::RefreshDisposition::Created
            | payload::RefreshDisposition::Promoted
            | payload::RefreshDisposition::CoalescedPending
            | payload::RefreshDisposition::Other(_) => Self::Pending,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(fake::Dummy))]
pub struct Feed {
    pub feed_type: Option<FeedType>,
    pub title: Option<String>,
    pub url: FeedUrl,
    pub updated: Option<Time>,
    pub links: Vec<Link>,
    pub website_url: Option<String>,
    pub description: Option<String>,
    pub generator: Option<String>,
    pub entries: Vec<EntryMeta>,
    pub authors: Vec<String>,
    pub refresh_policy: FeedRefreshPolicy,
    pub refresh_status: FeedRefreshStatus,
    requirement: Option<Requirement>,
    category: Option<Category<'static>>,
}

impl Feed {
    pub fn requirement(&self) -> Requirement {
        self.requirement.unwrap_or(ui::DEFAULT_REQUIREMNET)
    }

    pub fn category(&self) -> &Category<'static> {
        self.category.as_ref().unwrap_or(ui::default_category())
    }

    #[must_use]
    pub fn with_url(self, url: FeedUrl) -> Self {
        Self { url, ..self }
    }

    #[must_use]
    pub fn with_requirement(self, requirement: Requirement) -> Self {
        Self {
            requirement: Some(requirement),
            ..self
        }
    }

    #[must_use]
    pub fn with_category(self, category: Category<'static>) -> Self {
        Self {
            category: Some(category),
            ..self
        }
    }
}

impl From<payload::SubscribedFeed> for Feed {
    fn from(f: payload::SubscribedFeed) -> Self {
        let payload::SubscribedFeed {
            url,
            requirement,
            category,
            refresh_policy,
            refresh_status,
            feed: details,
        } = f;
        Self {
            feed_type: details
                .as_ref()
                .and_then(|details| details.feed_type.clone().into_feed_type()),
            title: details.as_ref().and_then(|details| details.title.clone()),
            url,
            updated: details
                .as_ref()
                .and_then(|details| details.updated.as_ref().map(parse_time)),
            links: details
                .as_ref()
                .map(|details| details.links.nodes.clone())
                .unwrap_or_default(),
            website_url: details
                .as_ref()
                .and_then(|details| details.website_url.clone()),
            description: details
                .as_ref()
                .and_then(|details| details.description.clone()),
            generator: details
                .as_ref()
                .and_then(|details| details.generator.clone()),
            entries: details
                .as_ref()
                .map(|details| details.entries.nodes.clone())
                .unwrap_or_default(),
            authors: details
                .as_ref()
                .map(|details| details.authors.nodes.clone())
                .unwrap_or_default(),
            refresh_policy: refresh_policy.into(),
            refresh_status: refresh_status.into(),
            requirement,
            category,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub title: Option<String>,
    pub published: Option<Time>,
    pub updated: Option<Time>,
    pub website_url: Option<String>,
    pub summary: Option<String>,
    pub feed_title: Option<String>,
    pub feed_url: FeedUrl,
    requirement: Option<Requirement>,
    category: Option<Category<'static>>,
}

impl Entry {
    pub fn summary_text(&self, width: usize) -> Option<String> {
        self.summary.as_deref().map(|summary| {
            html2text::config::plain()
                .string_from_read(summary.as_bytes(), width)
                .unwrap_or_default()
        })
    }

    pub fn requirement(&self) -> Requirement {
        self.requirement.unwrap_or(ui::DEFAULT_REQUIREMNET)
    }

    pub fn category(&self) -> &Category<'static> {
        self.category
            .as_ref()
            .unwrap_or_else(|| ui::default_category())
    }
}

impl From<payload::Entry> for Entry {
    fn from(v: payload::Entry) -> Self {
        Self {
            title: v.title,
            published: v.published.map(parse_time),
            updated: v.updated.map(parse_time),
            website_url: v.website_url,
            feed_title: v.feed.title,
            feed_url: v.feed.url,
            summary: v.summary,
            requirement: v.feed.requirement,
            category: v.feed.category,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportedFeed {
    pub title: Option<String>,
    pub url: FeedUrl,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    #[serde(default)]
    pub refresh_policy: Option<ExportedRefreshPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportedRefreshPolicy {
    pub kind: ExportedRefreshPolicyKind,
    pub interval_seconds: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExportedRefreshPolicyKind {
    Manual,
    Interval,
}

impl ExportedRefreshPolicy {
    fn from_api(value: &payload::RefreshPolicy) -> Option<Self> {
        match &value.kind {
            payload::RefreshPolicyKind::Manual => Some(Self {
                kind: ExportedRefreshPolicyKind::Manual,
                interval_seconds: None,
            }),
            payload::RefreshPolicyKind::Interval => Some(Self {
                kind: ExportedRefreshPolicyKind::Interval,
                interval_seconds: value.interval_seconds,
            }),
            payload::RefreshPolicyKind::Other(_) => None,
        }
    }
}

impl From<ExportedRefreshPolicy> for payload::RefreshPolicyInput {
    fn from(value: ExportedRefreshPolicy) -> Self {
        Self {
            kind: match value.kind {
                ExportedRefreshPolicyKind::Manual => payload::RefreshPolicyInputKind::Manual,
                ExportedRefreshPolicyKind::Interval => payload::RefreshPolicyInputKind::Interval,
            },
            interval_seconds: value.interval_seconds,
        }
    }
}

impl From<payload::SubscribedFeed> for ExportedFeed {
    fn from(v: payload::SubscribedFeed) -> Self {
        let refresh_policy = ExportedRefreshPolicy::from_api(&v.refresh_policy);
        Self {
            title: v.feed.and_then(|feed| feed.title),
            url: v.url,
            requirement: v.requirement,
            category: v.category,
            refresh_policy,
        }
    }
}

impl From<ExportedFeed> for crate::client::synd_api::payload::SubscribeFeedInput {
    fn from(feed: ExportedFeed) -> Self {
        Self {
            url: feed.url,
            requirement: feed.requirement,
            category: feed.category,
            refresh_policy: feed.refresh_policy.map(Into::into),
            initial_refresh: None,
        }
    }
}

fn parse_time(t: impl AsRef<str>) -> Time {
    DateTime::parse_from_rfc3339(t.as_ref())
        .expect("invalid rfc3339 time")
        .with_timezone(&chrono::Utc)
}
