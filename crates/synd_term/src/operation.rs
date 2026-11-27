use synd_auth::device_flow::DeviceAuthorizationResponse;
use synd_client::payload;
use synd_feed::types::{FeedUrl, SubscriptionLabels};
use url::Url;

use crate::{
    application::{Generation, Populate},
    auth::{AuthenticationProvider, Credential, Verified},
    client::gh::FetchNotificationsParams,
    types::gh::{NotificationDetail, NotificationId, ThreadId},
};

/// Identifies the UI that handles the outcome of a subscription write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SubscriptionOrigin {
    Form,
    Editor,
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

    OpenFeedEditionEditor {
        prompt: String,
        labels: SubscriptionLabels,
    },

    SubscribeFeed {
        input: payload::SubscribeFeedInput,
        origin: SubscriptionOrigin,
    },
    UnsubscribeFeed {
        url: FeedUrl,
    },
    FetchSubscription {
        generation: Generation,
        populate: Populate,
        after: Option<String>,
        first: i64,
    },
    OpenInitialTimeline,
    FetchTimelineWindow {
        timeline_id: payload::TimelineId,
        limit: usize,
    },
    CatchUpTimeline {
        timeline_id: payload::TimelineId,
        since: i64,
    },
    WatchFeedEvents,

    FetchGhNotifications {
        generation: Generation,
        populate: Populate,
        params: FetchNotificationsParams,
    },
    FetchGhDetail {
        generation: Generation,
        detail: NotificationDetail,
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
    FetchArticle {
        generation: Generation,
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
