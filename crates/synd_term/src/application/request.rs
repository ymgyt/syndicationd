use std::borrow::Cow;

use synd_client::SyndApiError;
use synd_feed::types::FeedUrl;

use crate::{
    auth::AuthenticationProvider,
    client::gh::GhError,
    types::gh::{IssueId, NotificationId, PullRequestId, ThreadId},
};

/// Opaque identity assigned to one logical external request.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct RequestId(u64);

impl RequestId {
    pub(super) fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Safe application-visible description of one logical external request.
#[derive(Debug)]
pub(crate) enum RequestKind {
    StartDeviceFlow { provider: AuthenticationProvider },
    PollDeviceFlowAccessToken { provider: AuthenticationProvider },
    SubscribeFeed { url: FeedUrl },
    UnsubscribeFeed { url: FeedUrl },
    FetchSubscription,
    FetchTimelineWindow { limit: usize },
    CatchUpTimeline { since: i64 },
    FetchGhNotifications { page: u8 },
    FetchGhIssue { id: IssueId },
    FetchGhPullRequest { id: PullRequestId },
    MarkGhNotificationAsDone { id: NotificationId },
    UnsubscribeGhThread { id: ThreadId },
}

impl RequestKind {
    pub(crate) fn label(&self) -> Cow<'static, str> {
        match self {
            Self::StartDeviceFlow { .. } => Cow::Borrowed("Request device authorization"),
            Self::PollDeviceFlowAccessToken { .. } => Cow::Borrowed("Poll device access token"),
            Self::SubscribeFeed { url } => Cow::Owned(format!("Subscribe feed {url}")),
            Self::UnsubscribeFeed { url } => Cow::Owned(format!("Unsubscribe feed {url}")),
            Self::FetchSubscription => Cow::Borrowed("Fetch subscriptions"),
            Self::FetchTimelineWindow { .. } => Cow::Borrowed("Fetch timeline"),
            Self::CatchUpTimeline { since } => {
                Cow::Owned(format!("Catch up timeline from {since}"))
            }
            Self::FetchGhNotifications { page } => {
                Cow::Owned(format!("Fetch GitHub notifications page {page}"))
            }
            Self::FetchGhIssue { id } => Cow::Owned(format!("Fetch GitHub issue #{id}")),
            Self::FetchGhPullRequest { id } => {
                Cow::Owned(format!("Fetch GitHub pull request #{id}"))
            }
            Self::MarkGhNotificationAsDone { id } => {
                Cow::Owned(format!("Mark GitHub notification {id} done"))
            }
            Self::UnsubscribeGhThread { id } => {
                Cow::Owned(format!("Unsubscribe GitHub thread {id}"))
            }
        }
    }
}

/// Failure of one registered logical request.
#[derive(Debug)]
pub(crate) enum RequestError {
    SyndApi(SyndApiError),
    Authentication(anyhow::Error),
    Gh(GhError),
}
