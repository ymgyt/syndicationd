use std::{fmt::Display, sync::Arc};

use synd_auth::device_flow::DeviceAuthorizationResponse;
use synd_client::{SyndApiError, payload};
use synd_feed::types::FeedUrl;

use crate::{
    application::{Populate, RequestSequence},
    auth::{AuthenticationProvider, Credential, Verified},
    client::github::GithubError,
    types::github::{IssueContext, Notification, NotificationId, PullRequestContext},
};

/// Successful external API result grouped by component domain.
#[derive(Debug, Clone)]
pub(crate) enum ApiEvent {
    Auth(AuthApiEvent),
    Feeds(FeedsApiEvent),
    GitHub(GitHubApiEvent),
}

#[derive(Debug, Clone)]
pub(crate) enum AuthApiEvent {
    DeviceFlowAuthorizationReceived {
        provider: AuthenticationProvider,
        device_authorization: Box<DeviceAuthorizationResponse>,
    },
    DeviceFlowCredentialReceived {
        credential: Verified<Credential>,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum FeedsApiEvent {
    FeedSubscribed,
    FeedUnsubscribed {
        url: FeedUrl,
    },
    SubscriptionFetched {
        populate: Populate,
        subscription: payload::SubscriptionPayload,
    },
    EntriesFetched {
        populate: Populate,
        payload: payload::TimelineEntryConnection,
    },
    TimelineChangesFetched {
        changes: Vec<payload::TimelineChange>,
        seq: i64,
    },
    InitialFeedViewFetched {
        payload: payload::InitialFeedViewPayload,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum GitHubApiEvent {
    NotificationsFetched {
        populate: Populate,
        notifications: Vec<Notification>,
    },
    IssueFetched {
        notification_id: NotificationId,
        issue: IssueContext,
    },
    PullRequestFetched {
        notification_id: NotificationId,
        pull_request: PullRequestContext,
    },
    NotificationMarkedAsDone {
        notification_id: NotificationId,
    },
    ThreadUnsubscribed,
}

impl Display for ApiEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiEvent::Auth(AuthApiEvent::DeviceFlowCredentialReceived { .. }) => {
                f.write_str("DeviceFlowCredentialReceived")
            }
            ApiEvent::Feeds(FeedsApiEvent::SubscriptionFetched { .. }) => {
                f.write_str("SubscriptionFetched")
            }
            ApiEvent::Feeds(FeedsApiEvent::EntriesFetched { .. }) => f.write_str("EntriesFetched"),
            ApiEvent::GitHub(GitHubApiEvent::NotificationsFetched { .. }) => {
                f.write_str("GithubNotificationsFetched")
            }
            ApiEvent::GitHub(GitHubApiEvent::IssueFetched { .. }) => {
                f.write_str("GithubIssueFetched")
            }
            ApiEvent::GitHub(GitHubApiEvent::PullRequestFetched { .. }) => {
                f.write_str("GithubPullRequestFetched")
            }
            event => write!(f, "{event:?}"),
        }
    }
}

/// Fact that already happened and can update application state.
#[derive(Debug, Clone)]
pub(crate) enum Event {
    TerminalResized,
    RenderThrobber,
    Idle,
    Api {
        request_seq: RequestSequence,
        event: ApiEvent,
    },
    CredentialRefreshed {
        credential: Verified<Credential>,
    },
    FeedSubscriptionEditorClosed {
        input: String,
    },
    FeedEditionEditorClosed {
        input: String,
    },
    EntryFetchStarted {
        request_seq: RequestSequence,
        populate: Populate,
    },
    RegistryFeed {
        event: payload::FeedEvent,
    },
    FeedEventSubscriptionInterrupted,
    FeedViewReloadDebounced {
        feeds_first: i64,
    },
    FeedViewSyncElapsed,
    TimelineSyncDebounced,
    Error {
        message: String,
    },
    SyndApiError {
        error: Arc<SyndApiError>,
        request_seq: RequestSequence,
    },
    OauthApiError {
        error: Arc<anyhow::Error>,
        request_seq: RequestSequence,
    },
    GithubApiError {
        error: Arc<GithubError>,
        request_seq: RequestSequence,
    },
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Api { event, .. } => event.fmt(f),
            _ => write!(f, "{self:?}"),
        }
    }
}

impl Event {
    pub(crate) fn synd_api_error(error: SyndApiError, request_seq: RequestSequence) -> Self {
        Event::SyndApiError {
            error: Arc::new(error),
            request_seq,
        }
    }

    pub(crate) fn oauth_api_error(error: anyhow::Error, request_seq: RequestSequence) -> Self {
        Event::OauthApiError {
            error: Arc::new(error),
            request_seq,
        }
    }
}
