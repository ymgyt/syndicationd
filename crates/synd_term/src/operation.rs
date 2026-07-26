use synd_auth::device_flow::DeviceAuthorizationResponse;
use synd_client::payload;
use synd_feed::types::FeedUrl;
use url::Url;

use crate::{
    application::Populate,
    auth::{AuthenticationProvider, Credential, Verified},
    client::gh::FetchNotificationsParams,
    types::gh::{
        IssueId, NotificationContext, NotificationDetail, NotificationId, PullRequestId, ThreadId,
    },
};

/// Ordered side effects requested by one application state transition.
#[derive(Debug)]
#[must_use]
pub(crate) enum Operations {
    Nop,
    One(Operation),
    Many(Vec<Operation>),
}

impl From<Vec<Operation>> for Operations {
    fn from(mut operations: Vec<Operation>) -> Self {
        match operations.len() {
            0 => Self::Nop,
            1 => Self::One(operations.pop().expect("length checked")),
            _ => Self::Many(operations),
        }
    }
}

impl From<Operation> for Operations {
    fn from(operation: Operation) -> Self {
        Self::One(operation)
    }
}

impl From<Option<Operation>> for Operations {
    fn from(operation: Option<Operation>) -> Self {
        match operation {
            Some(operation) => Self::One(operation),
            None => Self::Nop,
        }
    }
}

impl From<Vec<Operations>> for Operations {
    fn from(groups: Vec<Operations>) -> Self {
        let mut operations = Vec::new();
        for group in groups {
            match group {
                Self::Nop => {}
                Self::One(operation) => operations.push(operation),
                Self::Many(mut group) => operations.append(&mut group),
            }
        }
        operations.into()
    }
}

impl<const N: usize> From<[Operation; N]> for Operations {
    fn from(operations: [Operation; N]) -> Self {
        Vec::from(operations).into()
    }
}

impl<const N: usize> From<[Operations; N]> for Operations {
    fn from(groups: [Operations; N]) -> Self {
        Vec::from(groups).into()
    }
}

impl From<NotificationDetail> for Operation {
    fn from(detail: NotificationDetail) -> Self {
        match detail {
            NotificationDetail::Issue(context) => Self::FetchGhIssue { context },
            NotificationDetail::PullRequest(context) => Self::FetchGhPullRequest { context },
        }
    }
}

/// External side-effect request emitted by application state transitions.
#[derive(Debug)]
pub(crate) enum Operation {
    StartDeviceFlow {
        provider: AuthenticationProvider,
    },
    PollDeviceFlowAccessToken {
        provider: AuthenticationProvider,
        device_authorization: Box<DeviceAuthorizationResponse>,
    },

    OpenFeedSubscriptionEditor,
    OpenFeedEditionEditor {
        prompt: String,
    },

    SubscribeFeed {
        input: payload::SubscribeFeedInput,
    },
    UnsubscribeFeed {
        url: FeedUrl,
    },
    FetchSubscription {
        populate: Populate,
        after: Option<String>,
        first: i64,
    },
    FetchTimelineWindow {
        limit: usize,
    },
    CatchUpTimeline {
        since: i64,
    },
    WatchFeedEvents,

    FetchGhNotifications {
        populate: Populate,
        params: FetchNotificationsParams,
    },
    FetchGhIssue {
        context: NotificationContext<IssueId>,
    },
    FetchGhPullRequest {
        context: NotificationContext<PullRequestId>,
    },
    MarkGhNotificationAsDone {
        id: NotificationId,
    },
    UnsubscribeGhThread {
        id: ThreadId,
    },

    OpenBrowser {
        url: Url,
    },
    OpenTextBrowser {
        url: Url,
    },
    ForceRedrawTerminal,

    PersistCredential {
        credential: Verified<Credential>,
    },
    SetCredential {
        credential: Verified<Credential>,
    },
}
