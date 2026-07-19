use std::fmt::Display;
use synd_feed::types::Category;

use crate::{
    application::Direction,
    types::github::{PullRequestState, Reason},
    ui::widgets::{filter::FilterLane, gh_notifications::GhNotificationFilterUpdater},
};

/// Request for a component to perform an application action.
#[derive(Debug, Clone)]
pub(crate) enum Command {
    Nop,
    Shell(ShellCommand),
    Feeds(FeedsCommand),
    Filter(FilterCommand),
    GitHub(GitHubCommand),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ShellCommand {
    Quit,
    Authenticate,
    MoveAuthenticationProvider(Direction),
    MoveTabSelection(Direction),
    RotateTheme,
    ForceRedraw,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum FeedsCommand {
    MoveSubscribedFeed(Direction),
    MoveSubscribedFeedFirst,
    MoveSubscribedFeedLast,
    PromptFeedSubscription,
    PromptFeedEdition,
    PromptFeedUnsubscription,
    MoveFeedUnsubscriptionPopupSelection(Direction),
    SelectFeedUnsubscriptionPopup,
    CancelFeedUnsubscriptionPopup,
    ReloadSubscription,
    OpenFeed,

    // Entries
    ReloadEntries,
    MoveEntry(Direction),
    MoveEntryFirst,
    MoveEntryLast,
    OpenEntry,
    BrowseEntry,
}

#[derive(Debug, Clone)]
pub(crate) enum FilterCommand {
    MoveFilterRequirement(Direction),
    ActivateCategoryFilterling,
    ActivateSearchFiltering,
    PromptInsertChar(char),
    PromptDeleteBackward,
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
}

#[derive(Debug, Clone)]
pub(crate) enum GitHubCommand {
    MoveGhNotification(Direction),
    MoveGhNotificationFirst,
    MoveGhNotificationLast,
    OpenGhNotification { with_mark_as_done: bool },
    ReloadGhNotifications,
    MarkGhNotificationAsDone { all: bool },
    UnsubscribeGhThread,
    OpenGhNotificationFilterPopup,
    CloseGhNotificationFilterPopup,
    UpdateGhnotificationFilterPopupOptions(GhNotificationFilterUpdater),
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Command {
    pub fn nop() -> Self {
        Command::Nop
    }

    pub fn quit() -> Self {
        Command::Shell(ShellCommand::Quit)
    }
    pub fn force_redraw() -> Self {
        Command::Shell(ShellCommand::ForceRedraw)
    }
    pub fn authenticate() -> Self {
        Command::Shell(ShellCommand::Authenticate)
    }
    pub fn move_right_tab_selection() -> Self {
        Command::Shell(ShellCommand::MoveTabSelection(Direction::Right))
    }
    pub fn move_left_tab_selection() -> Self {
        Command::Shell(ShellCommand::MoveTabSelection(Direction::Left))
    }
    pub fn move_up_authentication_provider() -> Self {
        Command::Shell(ShellCommand::MoveAuthenticationProvider(Direction::Up))
    }
    pub fn move_down_authentication_provider() -> Self {
        Command::Shell(ShellCommand::MoveAuthenticationProvider(Direction::Down))
    }
    pub fn move_up_entry() -> Self {
        Command::Feeds(FeedsCommand::MoveEntry(Direction::Up))
    }
    pub fn move_down_entry() -> Self {
        Command::Feeds(FeedsCommand::MoveEntry(Direction::Down))
    }
    pub fn reload_entries() -> Self {
        Command::Feeds(FeedsCommand::ReloadEntries)
    }
    pub fn open_entry() -> Self {
        Command::Feeds(FeedsCommand::OpenEntry)
    }
    pub fn browse_entry() -> Self {
        Command::Feeds(FeedsCommand::BrowseEntry)
    }
    pub fn move_entry_first() -> Self {
        Command::Feeds(FeedsCommand::MoveEntryFirst)
    }
    pub fn move_entry_last() -> Self {
        Command::Feeds(FeedsCommand::MoveEntryLast)
    }
    pub fn prompt_feed_subscription() -> Self {
        Command::Feeds(FeedsCommand::PromptFeedSubscription)
    }
    pub fn prompt_feed_edition() -> Self {
        Command::Feeds(FeedsCommand::PromptFeedEdition)
    }
    pub fn prompt_feed_unsubscription() -> Self {
        Command::Feeds(FeedsCommand::PromptFeedUnsubscription)
    }
    pub fn move_feed_unsubscription_popup_selection_left() -> Self {
        Command::Feeds(FeedsCommand::MoveFeedUnsubscriptionPopupSelection(
            Direction::Left,
        ))
    }
    pub fn move_feed_unsubscription_popup_selection_right() -> Self {
        Command::Feeds(FeedsCommand::MoveFeedUnsubscriptionPopupSelection(
            Direction::Right,
        ))
    }
    pub fn select_feed_unsubscription_popup() -> Self {
        Command::Feeds(FeedsCommand::SelectFeedUnsubscriptionPopup)
    }
    pub fn cancel_feed_unsubscription_popup() -> Self {
        Command::Feeds(FeedsCommand::CancelFeedUnsubscriptionPopup)
    }
    pub fn move_up_subscribed_feed() -> Self {
        Command::Feeds(FeedsCommand::MoveSubscribedFeed(Direction::Up))
    }
    pub fn move_down_subscribed_feed() -> Self {
        Command::Feeds(FeedsCommand::MoveSubscribedFeed(Direction::Down))
    }
    pub fn reload_subscription() -> Self {
        Command::Feeds(FeedsCommand::ReloadSubscription)
    }
    pub fn open_feed() -> Self {
        Command::Feeds(FeedsCommand::OpenFeed)
    }
    pub fn move_subscribed_feed_first() -> Self {
        Command::Feeds(FeedsCommand::MoveSubscribedFeedFirst)
    }
    pub fn move_subscribed_feed_last() -> Self {
        Command::Feeds(FeedsCommand::MoveSubscribedFeedLast)
    }
    pub fn move_filter_requirement_left() -> Self {
        Command::Filter(FilterCommand::MoveFilterRequirement(Direction::Left))
    }
    pub fn move_filter_requirement_right() -> Self {
        Command::Filter(FilterCommand::MoveFilterRequirement(Direction::Right))
    }
    pub fn activate_category_filtering() -> Self {
        Command::Filter(FilterCommand::ActivateCategoryFilterling)
    }
    pub fn activate_search_filtering() -> Self {
        Command::Filter(FilterCommand::ActivateSearchFiltering)
    }
    pub fn deactivate_filtering() -> Self {
        Command::Filter(FilterCommand::DeactivateFiltering)
    }
    pub fn rotate_theme() -> Self {
        Command::Shell(ShellCommand::RotateTheme)
    }
    pub fn move_up_gh_notification() -> Self {
        Command::GitHub(GitHubCommand::MoveGhNotification(Direction::Up))
    }
    pub fn move_down_gh_notification() -> Self {
        Command::GitHub(GitHubCommand::MoveGhNotification(Direction::Down))
    }
    pub fn move_gh_notification_first() -> Self {
        Command::GitHub(GitHubCommand::MoveGhNotificationFirst)
    }
    pub fn move_gh_notification_last() -> Self {
        Command::GitHub(GitHubCommand::MoveGhNotificationLast)
    }
    pub fn open_gh_notification() -> Self {
        Command::GitHub(GitHubCommand::OpenGhNotification {
            with_mark_as_done: false,
        })
    }
    pub fn open_gh_notification_with_done() -> Self {
        Command::GitHub(GitHubCommand::OpenGhNotification {
            with_mark_as_done: true,
        })
    }
    pub fn reload_gh_notifications() -> Self {
        Command::GitHub(GitHubCommand::ReloadGhNotifications)
    }
    pub fn mark_gh_notification_as_done() -> Self {
        Command::GitHub(GitHubCommand::MarkGhNotificationAsDone { all: false })
    }
    pub fn mark_gh_notification_as_done_all() -> Self {
        Command::GitHub(GitHubCommand::MarkGhNotificationAsDone { all: true })
    }
    pub fn unsubscribe_gh_thread() -> Self {
        Command::GitHub(GitHubCommand::UnsubscribeGhThread)
    }
    pub fn open_gh_notification_filter_popup() -> Self {
        Command::GitHub(GitHubCommand::OpenGhNotificationFilterPopup)
    }
    pub fn close_gh_notification_filter_popup() -> Self {
        Command::GitHub(GitHubCommand::CloseGhNotificationFilterPopup)
    }
    pub fn toggle_gh_notification_filter_popup_include_unread() -> Self {
        Command::GitHub(GitHubCommand::UpdateGhnotificationFilterPopupOptions(
            GhNotificationFilterUpdater {
                toggle_include: true,
                ..Default::default()
            },
        ))
    }
    pub fn toggle_gh_notification_filter_popup_participating() -> Self {
        Command::GitHub(GitHubCommand::UpdateGhnotificationFilterPopupOptions(
            GhNotificationFilterUpdater {
                toggle_participating: true,
                ..Default::default()
            },
        ))
    }
    pub fn toggle_gh_notification_filter_popup_visibility_public() -> Self {
        Command::GitHub(GitHubCommand::UpdateGhnotificationFilterPopupOptions(
            GhNotificationFilterUpdater {
                toggle_visilibty_public: true,
                ..Default::default()
            },
        ))
    }
    pub fn toggle_gh_notification_filter_popup_visibility_private() -> Self {
        Command::GitHub(GitHubCommand::UpdateGhnotificationFilterPopupOptions(
            GhNotificationFilterUpdater {
                toggle_visilibty_private: true,
                ..Default::default()
            },
        ))
    }
    pub fn toggle_gh_notification_filter_popup_pr_open() -> Self {
        Command::GitHub(GitHubCommand::UpdateGhnotificationFilterPopupOptions(
            GhNotificationFilterUpdater {
                toggle_pull_request_condition: Some(PullRequestState::Open),
                ..Default::default()
            },
        ))
    }
    pub fn toggle_gh_notification_filter_popup_pr_closed() -> Self {
        Command::GitHub(GitHubCommand::UpdateGhnotificationFilterPopupOptions(
            GhNotificationFilterUpdater {
                toggle_pull_request_condition: Some(PullRequestState::Closed),
                ..Default::default()
            },
        ))
    }
    pub fn toggle_gh_notification_filter_popup_pr_merged() -> Self {
        Command::GitHub(GitHubCommand::UpdateGhnotificationFilterPopupOptions(
            GhNotificationFilterUpdater {
                toggle_pull_request_condition: Some(PullRequestState::Merged),
                ..Default::default()
            },
        ))
    }
    pub fn toggle_gh_notification_filter_popup_reason_mentioned() -> Self {
        Command::GitHub(GitHubCommand::UpdateGhnotificationFilterPopupOptions(
            GhNotificationFilterUpdater {
                toggle_reason: Some(Reason::Mention),
                ..Default::default()
            },
        ))
    }
    pub fn toggle_gh_notification_filter_popup_reason_review() -> Self {
        Command::GitHub(GitHubCommand::UpdateGhnotificationFilterPopupOptions(
            GhNotificationFilterUpdater {
                toggle_reason: Some(Reason::ReviewRequested),
                ..Default::default()
            },
        ))
    }
}
