use std::borrow::Cow;

use itertools::Itertools;
use synd_client::SyndApiError;
use synd_feed::types::FeedUrl;

use crate::{
    auth::AuthenticationProvider,
    client::gh::GhError,
    operation::SubscriptionOrigin,
    types::gh::{IssueId, NotificationId, PullRequestId, ThreadId},
};

pub(crate) use tuiframework::driver::RequestId;

/// Identifies results belonging to the current load of a view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Generation(u64);

impl Generation {
    pub(super) fn advance(&mut self) {
        self.0 = self.0.checked_add(1).expect("view generation exhausted");
    }
}

/// Safe application-visible description of one logical external request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RequestKind {
    StartDeviceFlow {
        provider: AuthenticationProvider,
    },
    PollDeviceFlowAccessToken {
        provider: AuthenticationProvider,
    },
    SubscribeFeed {
        url: FeedUrl,
        origin: SubscriptionOrigin,
    },
    UnsubscribeFeed {
        url: FeedUrl,
    },
    FetchSubscription {
        generation: Generation,
    },
    OpenInitialTimeline,
    FetchArticle {
        generation: Generation,
    },
    FetchTimelineWindow {
        limit: usize,
    },
    CatchUpTimeline {
        since: i64,
    },
    FetchGhNotifications {
        generation: Generation,
        page: u8,
    },
    FetchGhIssue {
        generation: Generation,
        id: IssueId,
    },
    FetchGhPullRequest {
        generation: Generation,
        id: PullRequestId,
    },
    MarkGhNotificationAsDone {
        id: NotificationId,
    },
    UnsubscribeGhThread {
        id: ThreadId,
    },
}

impl RequestKind {
    pub(crate) fn label(&self) -> Cow<'static, str> {
        match self {
            Self::StartDeviceFlow { .. } => Cow::Borrowed("Request device authorization"),
            Self::PollDeviceFlowAccessToken { .. } => Cow::Borrowed("Poll device access token"),
            Self::SubscribeFeed { url, .. } => Cow::Owned(format!("Subscribe feed {url}")),
            Self::UnsubscribeFeed { url } => Cow::Owned(format!("Unsubscribe feed {url}")),
            Self::FetchSubscription { .. } => Cow::Borrowed("Fetch subscriptions"),
            Self::OpenInitialTimeline => Cow::Borrowed("Open timeline"),
            Self::FetchArticle { .. } => Cow::Borrowed("Fetch article"),
            Self::FetchTimelineWindow { .. } => Cow::Borrowed("Fetch timeline"),
            Self::CatchUpTimeline { since } => {
                Cow::Owned(format!("Catch up timeline from {since}"))
            }
            Self::FetchGhNotifications { page, .. } => {
                Cow::Owned(format!("Fetch GitHub notifications page {page}"))
            }
            Self::FetchGhIssue { id, .. } => Cow::Owned(format!("Fetch GitHub issue #{id}")),
            Self::FetchGhPullRequest { id, .. } => {
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
    Article(anyhow::Error),
}

pub(super) fn synd_api_error_message(error: &SyndApiError) -> String {
    match error {
        SyndApiError::Unauthorized { url } => format!(
            "{} unauthorized. local feed API session is invalid",
            url.as_ref().map_or_default(ToString::to_string),
        ),
        SyndApiError::BuildRequest(error) => {
            format!("build request failed: {error} this is a BUG")
        }
        SyndApiError::SendRequest(error) => format!("request failed: {error}"),
        SyndApiError::DecodeResponse(error) => {
            format!("decode response failed: {error}")
        }
        SyndApiError::HttpStatus { status, url } => format!(
            "HTTP status client error ({status}) for url ({})",
            url.as_ref().map_or_default(ToString::to_string)
        ),
        SyndApiError::Graphql { errors } => errors.iter().map(ToString::to_string).join(", "),
        SyndApiError::OpenSession(error) => format!("session open rejected: {error:?}"),
        SyndApiError::RenewSession(error) => format!("session renew rejected: {error:?}"),
        SyndApiError::CloseSession(error) => format!("session close rejected: {error:?}"),
        _ => error.to_string(),
    }
}
