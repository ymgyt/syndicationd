use std::fmt::Display;
use synd_feed::types::{Category, FeedUrl};

use crate::{
    application::{ComponentId, Direction},
    reader::Scroll,
    types::gh::{PullRequestState, Reason, RepoVisibility},
};

/// An action requested by a key binding or a component.
#[derive(Debug, Clone)]
pub(crate) enum Command {
    Nop,
    /// Show the component that returned this command.
    Open,
    /// Hide the component that returned this command.
    Close,
    UnsubscribePopup(UnsubscribeCommand),
    GhFilterPopup(GhFilterCommand),
    List(ListCommand),
    Shell(ShellCommand),
    Login(LoginCommand),
    MoveTab(Direction),
    Feeds(FeedsCommand),
    Reader(ReaderCommand),
    Filter(FilterCommand),
    Gh(GhCommand),
}

impl Command {
    /// Shared list and overlay commands belong to the component that issued them.
    pub(crate) fn target(&self, owner: ComponentId) -> ComponentId {
        match self {
            Self::Nop | Self::Open | Self::Close | Self::List(_) => owner,
            Self::Shell(_) => ComponentId::Shell,
            Self::Login(_) => ComponentId::Login,
            Self::MoveTab(_) => ComponentId::Tabs,
            Self::Feeds(_) => ComponentId::Feeds,
            Self::Reader(_) => ComponentId::Reader,
            Self::Filter(_) => ComponentId::Filter,
            Self::Gh(_) => ComponentId::GhNotifications,
            Self::UnsubscribePopup(_) => ComponentId::UnsubscribePopup,
            Self::GhFilterPopup(_) => ComponentId::GhFilterPopup,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ListCommand {
    Previous,
    Next,
    First,
    Last,
}

#[derive(Debug, Clone)]
pub(crate) enum ShellCommand {
    ShowError(String),
    Quit,
    RotateTheme,
    ForceRedraw,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum LoginCommand {
    Authenticate,
    MoveProvider(Direction),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum FeedsCommand {
    PromptFeedSubscription,
    PromptFeedEdition,
    PromptFeedUnsubscription,
    ReloadSubscription,
    OpenFeed,

    RefreshTimeline,
    OpenEntry,
    ReadEntry,
    BrowseEntry,
}

#[derive(Debug, Clone)]
pub(crate) enum ReaderCommand {
    Open(url::Url),
    Close,
    Scroll(Scroll),
}

#[derive(Debug, Clone)]
pub(crate) struct UnsubscribeTarget {
    pub(crate) url: FeedUrl,
    pub(crate) title: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum UnsubscribeCommand {
    Open(UnsubscribeTarget),
    ToggleSelection,
    Select,
    Cancel,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum GhFilterCommand {
    Open,
    Close,
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

    ApplyNotificationFilter,
    ToggleNotificationFilter(GhNotificationFilterOption),
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
