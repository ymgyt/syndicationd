use std::{
    fmt::{self, Display},
    sync::Arc,
};

use crate::ui::widgets::{filter::FilterCriteria, tabs::Tab};
use synd_auth::device_flow::DeviceAuthorizationResponse;
use synd_client::{SyndApiError, payload};
use synd_feed::types::{Category, Requirement};
use synd_feed::types::{FeedUrl, SubscriptionLabels};
use synd_support::html::Article;
use url::Url;

use crate::{
    application::{Generation, PersistCacheError, Populate, RequestError, RequestId, RequestKind},
    auth::{AuthenticationProvider, Credential, CredentialError, Verified},
    external_tools::{OpenBrowserError, OpenEditorError},
    types::gh::{IssueContext, Notification, NotificationId, PullRequestContext},
};

/// Successful authentication fact produced by a registered request.
#[derive(Debug, Clone)]
pub(crate) enum AuthEvent {
    DeviceFlowAuthorizationReceived {
        provider: AuthenticationProvider,
        verification_url: Url,
        device_authorization: Box<DeviceAuthorizationResponse>,
    },
    DeviceFlowCredentialReceived {
        credential: Verified<Credential>,
    },
}

/// Feed facts, separated by request correlation shape.
#[derive(Debug)]
pub(crate) enum FeedsEvent {
    Request {
        request_id: RequestId,
        event: FeedRequestEvent,
    },
    Push {
        event: payload::FeedEvent,
    },
}

/// Successful feed fact produced by a registered request.
#[derive(Debug)]
pub(crate) enum FeedRequestEvent {
    FeedSubscribed {
        url: FeedUrl,
    },
    FeedUnsubscribed {
        url: FeedUrl,
    },
    SubscriptionFetched {
        generation: Generation,
        populate: Populate,
        subscription: payload::SubscriptionPayload,
    },
    InitialTimelineOpened {
        timeline_id: payload::TimelineId,
    },
    TimelineWindowChunkFetched {
        entries: Vec<payload::TimelineEntry>,
        base_seq: i64,
    },
    TimelineChangesFetched {
        changes: Vec<payload::TimelineChange>,
        seq: i64,
    },
}

/// Successful GitHub fact produced by a registered request.
#[derive(Debug)]
pub(crate) enum GhEvent {
    NotificationsFetched {
        generation: Generation,
        populate: Populate,
        notifications: Vec<Notification>,
    },
    IssueFetched {
        generation: Generation,
        notification_id: NotificationId,
        issue: IssueContext,
    },
    PullRequestFetched {
        generation: Generation,
        notification_id: NotificationId,
        pull_request: PullRequestContext,
    },
    NotificationMarkedAsDone {
        notification_id: NotificationId,
    },
}

/// Failure of a synchronous operation that is not a registered request.
#[derive(Debug)]
pub(crate) enum OperationError {
    OpenFeedEditionEditor(OpenEditorError),
    OpenBrowser(OpenBrowserError),
    OpenTextBrowser(OpenBrowserError),
    PersistCredential(PersistCacheError),
    SetCredential(SyndApiError),
}

/// Fact that already happened and can update application state.
#[derive(Debug, strum::EnumDiscriminants)]
#[strum_discriminants(name(EventKind), derive(Hash))]
pub(crate) enum Event {
    TabChanged(Tab),
    FeedFilterChanged {
        requirement: Requirement,
        criteria: FilterCriteria,
    },
    GhFilterChanged(FilterCriteria),
    FeedCategoriesChanged(Vec<Category<'static>>),
    GhCategoriesChanged {
        populate: Populate,
        categories: Vec<Category<'static>>,
    },
    ApiAccessReady,
    /// A replacement credential is now in use for an already authenticated API.
    ApiAccessRefreshed,
    RequestEmitted {
        request_id: RequestId,
        kind: RequestKind,
    },
    RequestCompleted {
        request_id: RequestId,
        kind: RequestKind,
        result: Result<(), RequestError>,
    },

    Auth {
        request_id: RequestId,
        event: AuthEvent,
    },
    Feeds(FeedsEvent),
    ArticleFetched {
        request_id: RequestId,
        generation: Generation,
        article: Arc<Article>,
    },
    Gh {
        request_id: RequestId,
        event: GhEvent,
    },

    FeedEditionEditorClosed {
        input: String,
        labels: SubscriptionLabels,
    },

    ApiCredentialConfigured,
    CredentialRefreshed {
        credential: Verified<Credential>,
    },
    CredentialRefreshFailed {
        error: CredentialError,
    },

    OperationFailed {
        error: OperationError,
    },
}

impl Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FeedFilterChanged { .. } => f.write_str("FeedFilterChanged"),
            Self::GhFilterChanged(_) => f.write_str("GhFilterChanged"),
            Self::FeedCategoriesChanged(_) => f.write_str("FeedCategoriesChanged"),
            Self::GhCategoriesChanged { .. } => f.write_str("GhCategoriesChanged"),
            Self::ApiAccessRefreshed => f.write_str("ApiAccessRefreshed"),
            Self::ApiAccessReady => f.write_str("ApiAccessReady"),
            Self::TabChanged(tab) => write!(f, "TabChanged({tab:?})"),
            Self::RequestEmitted { request_id, kind } => {
                write!(f, "RequestEmitted({request_id:?}, {kind:?})")
            }
            Self::RequestCompleted {
                request_id,
                result: Ok(()),
                ..
            } => write!(f, "RequestCompleted({request_id:?}, Ok)"),
            Self::RequestCompleted {
                request_id,
                result: Err(_),
                ..
            } => write!(f, "RequestCompleted({request_id:?}, Err)"),
            Self::Auth { request_id, event } => {
                write!(f, "Auth({request_id:?}, {})", event.name())
            }
            Self::Feeds(FeedsEvent::Request { request_id, event }) => {
                write!(f, "Feeds::Request({request_id:?}, {})", event.name())
            }
            Self::Feeds(FeedsEvent::Push { .. }) => f.write_str("Feeds::Push"),
            Self::ArticleFetched { request_id, .. } => write!(f, "ArticleFetched({request_id:?})"),
            Self::Gh {
                request_id, event, ..
            } => {
                write!(f, "Gh({request_id:?}, {})", event.name())
            }
            Self::FeedEditionEditorClosed { .. } => f.write_str("FeedEditionEditorClosed"),
            Self::ApiCredentialConfigured => f.write_str("ApiCredentialConfigured"),
            Self::CredentialRefreshed { .. } => f.write_str("CredentialRefreshed"),
            Self::CredentialRefreshFailed { .. } => f.write_str("CredentialRefreshFailed"),
            Self::OperationFailed { error } => {
                write!(f, "OperationFailed({})", error.name())
            }
        }
    }
}

impl AuthEvent {
    fn name(&self) -> &'static str {
        match self {
            Self::DeviceFlowAuthorizationReceived { .. } => "DeviceFlowAuthorizationReceived",
            Self::DeviceFlowCredentialReceived { .. } => "DeviceFlowCredentialReceived",
        }
    }
}

impl FeedRequestEvent {
    fn name(&self) -> &'static str {
        match self {
            Self::FeedSubscribed { .. } => "FeedSubscribed",
            Self::FeedUnsubscribed { .. } => "FeedUnsubscribed",
            Self::SubscriptionFetched { .. } => "SubscriptionFetched",
            Self::InitialTimelineOpened { .. } => "InitialTimelineOpened",
            Self::TimelineWindowChunkFetched { .. } => "TimelineWindowChunkFetched",
            Self::TimelineChangesFetched { .. } => "TimelineChangesFetched",
        }
    }
}

impl GhEvent {
    fn name(&self) -> &'static str {
        match self {
            Self::NotificationsFetched { .. } => "NotificationsFetched",
            Self::IssueFetched { .. } => "IssueFetched",
            Self::PullRequestFetched { .. } => "PullRequestFetched",
            Self::NotificationMarkedAsDone { .. } => "NotificationMarkedAsDone",
        }
    }
}

impl OperationError {
    fn name(&self) -> &'static str {
        match self {
            Self::OpenFeedEditionEditor(_) => "OpenFeedEditionEditor",
            Self::OpenBrowser(_) => "OpenBrowser",
            Self::OpenTextBrowser(_) => "OpenTextBrowser",
            Self::PersistCredential(_) => "PersistCredential",
            Self::SetCredential(_) => "SetCredential",
        }
    }
}
