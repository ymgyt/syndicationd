use synd_auth::device_flow::DeviceAuthorizationResponse;
use synd_client::payload;
use synd_feed::types::FeedUrl;
use url::Url;

use crate::{
    application::Populate,
    auth::AuthenticationProvider,
    client::github::FetchNotificationsParams,
    types::github::{IssueOrPullRequest, NotificationId, ThreadId},
};

/// External side-effect request emitted by components and executed by drivers.
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
    FetchSubscription {
        populate: Populate,
        after: Option<String>,
        first: i64,
    },
    FetchEntries {
        populate: Populate,
        after: Option<String>,
        first: i64,
    },
    /// Fetch timeline changes after `since` and apply them
    SyncTimeline {
        since: i64,
    },
    StartFeedEventSubscription,
    ScheduleFeedViewReload {
        feeds_first: i64,
    },
    UnsubscribeFeed {
        url: FeedUrl,
    },
    ScheduleFeedViewSync,
    ScheduleTimelineSync,
    FetchGitHubNotifications {
        populate: Populate,
        params: FetchNotificationsParams,
    },
    FetchGitHubNotificationDetails {
        contexts: Vec<IssueOrPullRequest>,
    },
    MarkGitHubNotificationAsDone {
        id: NotificationId,
    },
    UnsubscribeGitHubThread {
        id: ThreadId,
    },
    OpenBrowser {
        url: Url,
    },
    OpenTextBrowser {
        url: Url,
    },
    ForceRedrawTerminal,
}
