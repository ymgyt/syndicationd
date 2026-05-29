use std::fmt;

use chrono::{DateTime, Utc};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use synd_feed::types::FeedUrl;

use super::SubscriberId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RefreshRequestId(String);

impl RefreshRequestId {
    pub fn generate() -> Self {
        Self(Alphanumeric.sample_string(&mut rand::rng(), 24))
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RefreshRequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RefreshPriority {
    Background,
    Immediate,
    Interactive,
}

impl RefreshPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Immediate => "immediate",
            Self::Interactive => "interactive",
        }
    }
}

impl TryFrom<&str> for RefreshPriority {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "background" => Ok(Self::Background),
            "immediate" => Ok(Self::Immediate),
            "interactive" => Ok(Self::Interactive),
            value => Err(anyhow::anyhow!("unknown refresh priority: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefreshIntentKind {
    Initial,
    Scheduled,
    Manual,
    PolicyChanged,
    Startup,
}

impl RefreshIntentKind {
    pub fn priority(self) -> RefreshPriority {
        match self {
            Self::Manual => RefreshPriority::Interactive,
            Self::Initial | Self::PolicyChanged => RefreshPriority::Immediate,
            Self::Scheduled | Self::Startup => RefreshPriority::Background,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Scheduled => "scheduled",
            Self::Manual => "manual",
            Self::PolicyChanged => "policy_changed",
            Self::Startup => "startup",
        }
    }
}

impl TryFrom<&str> for RefreshIntentKind {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "initial" => Ok(Self::Initial),
            "scheduled" => Ok(Self::Scheduled),
            "manual" => Ok(Self::Manual),
            "policy_changed" => Ok(Self::PolicyChanged),
            "startup" => Ok(Self::Startup),
            value => Err(anyhow::anyhow!("unknown refresh intent: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefreshRequestStatus {
    Pending,
    Running,
}

impl RefreshRequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
        }
    }
}

impl TryFrom<&str> for RefreshRequestStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            value => Err(anyhow::anyhow!("unknown refresh request status: {value}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefreshIntent {
    pub feed_url: FeedUrl,
    pub intent: RefreshIntentKind,
    pub priority: RefreshPriority,
    pub requested_by: Option<SubscriberId>,
    pub requested_at: DateTime<Utc>,
    pub not_before: DateTime<Utc>,
}

impl RefreshIntent {
    pub fn new(
        feed_url: FeedUrl,
        intent: RefreshIntentKind,
        requested_by: Option<SubscriberId>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            feed_url,
            intent,
            priority: intent.priority(),
            requested_by,
            requested_at: now,
            not_before: now,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewRefreshRequest {
    pub id: RefreshRequestId,
    pub feed_url: FeedUrl,
    pub intent: RefreshIntentKind,
    pub priority: RefreshPriority,
    pub requested_by: Option<SubscriberId>,
    pub requested_at: Option<DateTime<Utc>>,
    pub signal_count: i64,
    pub not_before: DateTime<Utc>,
    pub status: RefreshRequestStatus,
    pub attempt_count: i64,
    pub lease_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<RefreshIntent> for NewRefreshRequest {
    fn from(intent: RefreshIntent) -> Self {
        let now = intent.requested_at;
        Self {
            id: RefreshRequestId::generate(),
            feed_url: intent.feed_url,
            intent: intent.intent,
            priority: intent.priority,
            requested_by: intent.requested_by,
            requested_at: Some(intent.requested_at),
            signal_count: 1,
            not_before: intent.not_before,
            status: RefreshRequestStatus::Pending,
            attempt_count: 0,
            lease_until: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefreshRequest {
    pub id: RefreshRequestId,
    pub feed_url: FeedUrl,
    pub intent: RefreshIntentKind,
    pub priority: RefreshPriority,
    pub requested_by: Option<SubscriberId>,
    pub requested_at: Option<DateTime<Utc>>,
    pub signal_count: i64,
    pub not_before: DateTime<Utc>,
    pub status: RefreshRequestStatus,
    pub attempt_count: i64,
    pub lease_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RefreshRequestUpdate {
    pub id: RefreshRequestId,
    pub feed_url: FeedUrl,
    pub intent: RefreshIntentKind,
    pub priority: RefreshPriority,
    pub requested_by: Option<SubscriberId>,
    pub requested_at: Option<DateTime<Utc>>,
    pub signal_count: i64,
    pub not_before: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RefreshRequestUpdate {
    pub fn from_merge(active: &RefreshRequest, incoming: &RefreshIntent) -> Self {
        let promote = incoming.priority > active.priority;
        Self {
            id: active.id.clone(),
            feed_url: active.feed_url.clone(),
            intent: if promote {
                incoming.intent
            } else {
                active.intent
            },
            priority: active.priority.max(incoming.priority),
            requested_by: incoming
                .requested_by
                .clone()
                .or_else(|| active.requested_by.clone()),
            requested_at: Some(incoming.requested_at),
            signal_count: active.signal_count.saturating_add(1),
            not_before: active.not_before.min(incoming.not_before),
            updated_at: incoming.requested_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClaimedRefreshRequest {
    pub id: RefreshRequestId,
    pub feed_url: FeedUrl,
    pub lease_until: DateTime<Utc>,
    pub attempt_count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshRequestDisposition {
    Created,
    Promoted,
    CoalescedPending,
    JoinedRunning,
}

#[derive(Debug, Clone)]
pub struct RefreshRequestReceipt {
    pub request_id: RefreshRequestId,
    pub disposition: RefreshRequestDisposition,
    pub status: RefreshStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshStatusKind {
    NeverRefreshed,
    Idle,
    Pending,
    Running,
    LastFailed,
}

#[derive(Debug, Clone)]
pub struct RefreshStatus {
    pub feed_url: FeedUrl,
    pub kind: RefreshStatusKind,
    pub active_request_id: Option<RefreshRequestId>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_error_message: Option<String>,
}
