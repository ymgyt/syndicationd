use std::fmt::Display;
use synd_feed::types::Category;

use crate::{
    application::{ComponentId, Direction},
    types::gh::{PullRequestState, Reason, RepoVisibility},
};

/// An action requested by a key binding or a component.
#[derive(Debug, Clone)]
pub(crate) enum Command {
    Nop,
    Open(ComponentId),
    Close(ComponentId),
    List {
        to: ComponentId,
        command: ListCommand,
    },
    Shell(ShellCommand),
    Feeds(FeedsCommand),
    Filter(FilterCommand),
    Gh(GhCommand),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ListCommand {
    Previous,
    Next,
    First,
    Last,
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
    PromptFeedSubscription,
    PromptFeedEdition,
    PromptFeedUnsubscription,
    MoveFeedUnsubscriptionPopupSelection(Direction),
    SelectFeedUnsubscriptionPopup,
    CancelFeedUnsubscriptionPopup,
    ReloadSubscription,
    OpenFeed,

    RefreshTimeline,
    OpenEntry,
    BrowseEntry,
}

#[derive(Debug, Clone)]
pub(crate) enum FilterCommand {
    MoveFilterRequirement(Direction),
    ActivateCategoryFiltering,
    ActivateSearchFiltering,
    PromptInsertChar(char),
    PromptDeleteBackward,
    DeactivateFiltering,
    ToggleFilterCategory {
        target: FilterTarget,
        category: Category<'static>,
    },
    ActivateAllFilterCategories {
        target: FilterTarget,
    },
    DeactivateAllFilterCategories {
        target: FilterTarget,
    },
}

/// Application concern whose filter state is being changed or rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterTarget {
    Feeds,
    GhNotifications,
}

/// One GitHub notification-filter choice toggled by a user command.
#[derive(Debug, Clone)]
pub(crate) enum GhNotificationFilterOption {
    UnreadOnly,
    ParticipatingOnly,
    Visibility(RepoVisibility),
    PullRequestState(PullRequestState),
    Reason(Reason),
}

#[derive(Debug, Clone)]
pub(crate) enum GhCommand {
    OpenNotification,
    OpenNotificationAndMarkAsDone,
    ReloadNotifications,
    MarkNotificationAsDone,
    MarkAllNotificationsAsDone,
    UnsubscribeThread,

    OpenNotificationFilter,
    CloseNotificationFilter,
    ToggleNotificationFilter(GhNotificationFilterOption),
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
