use std::{fmt::Display, sync::Arc};
use synd_auth::device_flow::DeviceAuthorizationResponse;
use synd_feed::types::{Category, FeedUrl};

use crate::{
    application::{Direction, Populate, RequestSequence},
    auth::{AuthenticationProvider, Credential, Verified},
    client::{
        github::{FetchNotificationsParams, GithubError},
        synd_api::{
            SyndApiError, mutation::subscribe_feed::SubscribeFeedInput, payload,
            query::subscription::SubscriptionOutput,
        },
    },
    types::{
        Feed,
        github::{
            IssueContext, IssueOrPullRequest, Notification, NotificationId, PullRequestContext,
            PullRequestState, Reason,
        },
    },
    ui::components::{filter::FilterLane, gh_notifications::GhNotificationFilterUpdater},
};

#[derive(Debug, Clone)]
pub(crate) enum ApiResponse {
    DeviceFlowAuthorization {
        provider: AuthenticationProvider,
        device_authorization: DeviceAuthorizationResponse,
    },
    DeviceFlowCredential {
        credential: Verified<Credential>,
    },
    SubscribeFeed {
        feed: Box<Feed>,
    },
    UnsubscribeFeed {
        url: FeedUrl,
    },
    FetchSubscription {
        populate: Populate,
        subscription: SubscriptionOutput,
    },
    FetchEntries {
        populate: Populate,
        payload: payload::FetchEntriesPayload,
    },
    FetchGithubNotifications {
        populate: Populate,
        notifications: Vec<Notification>,
    },
    FetchGithubIssue {
        notification_id: NotificationId,
        issue: IssueContext,
    },
    FetchGithubPullRequest {
        notification_id: NotificationId,
        pull_request: PullRequestContext,
    },
    MarkGithubNotificationAsDone {
        notification_id: NotificationId,
    },
    UnsubscribeGithubThread {},
}

impl Display for ApiResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiResponse::DeviceFlowCredential { .. } => f.write_str("DeviceFlowCredential"),
            ApiResponse::FetchSubscription { .. } => f.write_str("FetchSubscription"),
            ApiResponse::FetchEntries { .. } => f.write_str("FetchEntries"),
            ApiResponse::FetchGithubNotifications { .. } => f.write_str("FetchGithubNotifications"),
            ApiResponse::FetchGithubIssue { .. } => f.write_str("FetchGithubIssue"),
            ApiResponse::FetchGithubPullRequest { .. } => f.write_str("FetchGithubPullRequest"),
            cmd => write!(f, "{cmd:?}"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Command {
    Nop,
    Quit,
    ResizeTerminal {
        _columns: u16,
        _rows: u16,
    },
    RenderThrobber,
    Idle,

    Authenticate,
    MoveAuthenticationProvider(Direction),

    HandleApiResponse {
        request_seq: RequestSequence,
        response: ApiResponse,
    },

    RefreshCredential {
        credential: Verified<Credential>,
    },

    MoveTabSelection(Direction),

    // Subscription
    MoveSubscribedFeed(Direction),
    MoveSubscribedFeedFirst,
    MoveSubscribedFeedLast,
    PromptFeedSubscription,
    PromptFeedEdition,
    PromptFeedUnsubscription,
    MoveFeedUnsubscriptionPopupSelection(Direction),
    SelectFeedUnsubscriptionPopup,
    CancelFeedUnsubscriptionPopup,
    SubscribeFeed {
        input: SubscribeFeedInput,
    },
    FetchSubscription {
        after: Option<String>,
        first: i64,
    },
    ReloadSubscription,
    OpenFeed,

    // Entries
    FetchEntries {
        after: Option<String>,
        first: i64,
    },
    ReloadEntries,
    MoveEntry(Direction),
    MoveEntryFirst,
    MoveEntryLast,
    OpenEntry,
    BrowseEntry,

    // Filter
    MoveFilterRequirement(Direction),
    ActivateCategoryFilterling,
    ActivateSearchFiltering,
    PromptChanged,
    DeactivateFiltering,
    ToggleFilterCategory {
        lane: FilterLane,
        category: Category<'static>,
    },
    ActivateAllFilterCategories {
        lane: FilterLane,
    },
    DeactivateAllFilterCategories {
        lane: FilterLane,
    },

    // Theme
    RotateTheme,

    // Latest release check
    InformLatestRelease(update_informer::Version),

    // Github notifications
    FetchGhNotifications {
        populate: Populate,
        params: FetchNotificationsParams,
    },
    FetchGhNotificationDetails {
        contexts: Vec<IssueOrPullRequest>,
    },
    MoveGhNotification(Direction),
    MoveGhNotificationFirst,
    MoveGhNotificationLast,
    OpenGhNotification {
        with_mark_as_done: bool,
    },
    ReloadGhNotifications,
    MarkGhNotificationAsDone {
        all: bool,
    },
    UnsubscribeGhThread,
    OpenGhNotificationFilterPopup,
    CloseGhNotificationFilterPopup,
    UpdateGhnotificationFilterPopupOptions(GhNotificationFilterUpdater),

    // Error
    HandleError {
        message: String,
    },
    HandleApiError {
        // use Arc for impl Clone
        error: Arc<SyndApiError>,
        request_seq: RequestSequence,
    },
    HandleOauthApiError {
        // use Arc for impl Clone
        error: Arc<anyhow::Error>,
        request_seq: RequestSequence,
    },
    HandleGithubApiError {
        // use Arc for impl Clone
        error: Arc<GithubError>,
        request_seq: RequestSequence,
    },
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::HandleApiResponse { response, .. } => response.fmt(f),
            Command::FetchGhNotificationDetails { .. } => f
                .debug_struct("FetchGhNotificationDetails")
                .finish_non_exhaustive(),
            _ => write!(f, "{self:?}"),
        }
    }
}

impl Command {
    pub(crate) fn api_error(error: SyndApiError, request_seq: RequestSequence) -> Self {
        Command::HandleApiError {
            error: Arc::new(error),
            request_seq,
        }
    }
    pub(crate) fn oauth_api_error(error: anyhow::Error, request_seq: RequestSequence) -> Self {
        Command::HandleOauthApiError {
            error: Arc::new(error),
            request_seq,
        }
    }
}

impl Command {
    pub fn quit() -> Self {
        Command::Quit
    }
    pub fn authenticate() -> Self {
        Command::Authenticate
    }
    pub fn move_right_tab_selection() -> Self {
        Command::MoveTabSelection(Direction::Right)
    }
    pub fn move_left_tab_selection() -> Self {
        Command::MoveTabSelection(Direction::Left)
    }
    pub fn move_up_authentication_provider() -> Self {
        Command::MoveAuthenticationProvider(Direction::Up)
    }
    pub fn move_down_authentication_provider() -> Self {
        Command::MoveAuthenticationProvider(Direction::Down)
    }
    pub fn move_up_entry() -> Self {
        Command::MoveEntry(Direction::Up)
    }
    pub fn move_down_entry() -> Self {
        Command::MoveEntry(Direction::Down)
    }
    pub fn reload_entries() -> Self {
        Command::ReloadEntries
    }
    pub fn open_entry() -> Self {
        Command::OpenEntry
    }
    pub fn browse_entry() -> Self {
        Command::BrowseEntry
    }
    pub fn move_entry_first() -> Self {
        Command::MoveEntryFirst
    }
    pub fn move_entry_last() -> Self {
        Command::MoveEntryLast
    }
    pub fn prompt_feed_subscription() -> Self {
        Command::PromptFeedSubscription
    }
    pub fn prompt_feed_edition() -> Self {
        Command::PromptFeedEdition
    }
    pub fn prompt_feed_unsubscription() -> Self {
        Command::PromptFeedUnsubscription
    }
    pub fn move_feed_unsubscription_popup_selection_left() -> Self {
        Command::MoveFeedUnsubscriptionPopupSelection(Direction::Left)
    }
    pub fn move_feed_unsubscription_popup_selection_right() -> Self {
        Command::MoveFeedUnsubscriptionPopupSelection(Direction::Right)
    }
    pub fn select_feed_unsubscription_popup() -> Self {
        Command::SelectFeedUnsubscriptionPopup
    }
    pub fn cancel_feed_unsubscription_popup() -> Self {
        Command::CancelFeedUnsubscriptionPopup
    }
    pub fn move_up_subscribed_feed() -> Self {
        Command::MoveSubscribedFeed(Direction::Up)
    }
    pub fn move_down_subscribed_feed() -> Self {
        Command::MoveSubscribedFeed(Direction::Down)
    }
    pub fn reload_subscription() -> Self {
        Command::ReloadSubscription
    }
    pub fn open_feed() -> Self {
        Command::OpenFeed
    }
    pub fn move_subscribed_feed_first() -> Self {
        Command::MoveSubscribedFeedFirst
    }
    pub fn move_subscribed_feed_last() -> Self {
        Command::MoveSubscribedFeedLast
    }
    pub fn move_filter_requirement_left() -> Self {
        Command::MoveFilterRequirement(Direction::Left)
    }
    pub fn move_filter_requirement_right() -> Self {
        Command::MoveFilterRequirement(Direction::Right)
    }
    pub fn activate_category_filtering() -> Self {
        Command::ActivateCategoryFilterling
    }
    pub fn activate_search_filtering() -> Self {
        Command::ActivateSearchFiltering
    }
    pub fn deactivate_filtering() -> Self {
        Command::DeactivateFiltering
    }
    pub fn rotate_theme() -> Self {
        Command::RotateTheme
    }
    pub fn move_up_gh_notification() -> Self {
        Command::MoveGhNotification(Direction::Up)
    }
    pub fn move_down_gh_notification() -> Self {
        Command::MoveGhNotification(Direction::Down)
    }
    pub fn move_gh_notification_first() -> Self {
        Command::MoveGhNotificationFirst
    }
    pub fn move_gh_notification_last() -> Self {
        Command::MoveGhNotificationLast
    }
    pub fn open_gh_notification() -> Self {
        Command::OpenGhNotification {
            with_mark_as_done: false,
        }
    }
    pub fn open_gh_notification_with_done() -> Self {
        Command::OpenGhNotification {
            with_mark_as_done: true,
        }
    }
    pub fn reload_gh_notifications() -> Self {
        Command::ReloadGhNotifications
    }
    pub fn mark_gh_notification_as_done() -> Self {
        Command::MarkGhNotificationAsDone { all: false }
    }
    pub fn mark_gh_notification_as_done_all() -> Self {
        Command::MarkGhNotificationAsDone { all: true }
    }
    pub fn unsubscribe_gh_thread() -> Self {
        Command::UnsubscribeGhThread
    }
    pub fn open_gh_notification_filter_popup() -> Self {
        Command::OpenGhNotificationFilterPopup
    }
    pub fn close_gh_notification_filter_popup() -> Self {
        Command::CloseGhNotificationFilterPopup
    }
    pub fn toggle_gh_notification_filter_popup_include_unread() -> Self {
        Command::UpdateGhnotificationFilterPopupOptions(GhNotificationFilterUpdater {
            toggle_include: true,
            ..Default::default()
        })
    }
    pub fn toggle_gh_notification_filter_popup_participating() -> Self {
        Command::UpdateGhnotificationFilterPopupOptions(GhNotificationFilterUpdater {
            toggle_participating: true,
            ..Default::default()
        })
    }
    pub fn toggle_gh_notification_filter_popup_visibility_public() -> Self {
        Command::UpdateGhnotificationFilterPopupOptions(GhNotificationFilterUpdater {
            toggle_visilibty_public: true,
            ..Default::default()
        })
    }
    pub fn toggle_gh_notification_filter_popup_visibility_private() -> Self {
        Command::UpdateGhnotificationFilterPopupOptions(GhNotificationFilterUpdater {
            toggle_visilibty_private: true,
            ..Default::default()
        })
    }
    pub fn toggle_gh_notification_filter_popup_pr_open() -> Self {
        Command::UpdateGhnotificationFilterPopupOptions(GhNotificationFilterUpdater {
            toggle_pull_request_condition: Some(PullRequestState::Open),
            ..Default::default()
        })
    }
    pub fn toggle_gh_notification_filter_popup_pr_closed() -> Self {
        Command::UpdateGhnotificationFilterPopupOptions(GhNotificationFilterUpdater {
            toggle_pull_request_condition: Some(PullRequestState::Closed),
            ..Default::default()
        })
    }
    pub fn toggle_gh_notification_filter_popup_pr_merged() -> Self {
        Command::UpdateGhnotificationFilterPopupOptions(GhNotificationFilterUpdater {
            toggle_pull_request_condition: Some(PullRequestState::Merged),
            ..Default::default()
        })
    }
    pub fn toggle_gh_notification_filter_popup_reason_mentioned() -> Self {
        Command::UpdateGhnotificationFilterPopupOptions(GhNotificationFilterUpdater {
            toggle_reason: Some(Reason::Mention),
            ..Default::default()
        })
    }
    pub fn toggle_gh_notification_filter_popup_reason_review() -> Self {
        Command::UpdateGhnotificationFilterPopupOptions(GhNotificationFilterUpdater {
            toggle_reason: Some(Reason::ReviewRequested),
            ..Default::default()
        })
    }
}
