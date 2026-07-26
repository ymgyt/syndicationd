use std::{fmt, str::FromStr};

use crate::{
    application::Direction,
    command::{
        Command, FeedsCommand, FilterCommand, GhCommand, GhNotificationFilterOption, ShellCommand,
    },
    types::gh::{PullRequestState, Reason, RepoVisibility},
};
use serde::Deserialize;

use super::{KeyBinding, KeymapError, Layer};

/// Stable command identifier accepted by keymap configuration.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CommandId {
    #[default]
    Nop,
    Quit,
    ForceRedraw,
    RotateTheme,
    Authenticate,
    MoveAuthenticationProviderPrev,
    MoveAuthenticationProviderNext,
    MoveTabPrev,
    MoveTabNext,
    MoveEntryPrev,
    MoveEntryNext,
    MoveEntryFirst,
    MoveEntryLast,
    RefreshTimeline,
    OpenEntry,
    BrowseEntry,
    MoveSubscribedFeedPrev,
    MoveSubscribedFeedNext,
    MoveSubscribedFeedFirst,
    MoveSubscribedFeedLast,
    PromptFeedSubscription,
    PromptFeedEdition,
    PromptFeedUnsubscription,
    ReloadSubscription,
    OpenFeed,
    MoveFeedUnsubscriptionPopupSelectionPrev,
    MoveFeedUnsubscriptionPopupSelectionNext,
    SelectFeedUnsubscriptionPopup,
    CancelFeedUnsubscriptionPopup,
    MoveFilterRequirementPrev,
    MoveFilterRequirementNext,
    ActivateCategoryFiltering,
    ActivateSearchFiltering,
    DeactivateFiltering,
    MoveGhNotificationPrev,
    MoveGhNotificationNext,
    MoveGhNotificationFirst,
    MoveGhNotificationLast,
    OpenGhNotification,
    OpenGhNotificationAndMarkAsDone,
    ReloadGhNotifications,
    MarkGhNotificationAsDone,
    MarkAllGhNotificationsAsDone,
    UnsubscribeGhThread,
    OpenGhNotificationFilter,
    CloseGhNotificationFilter,
    ToggleGhNotificationFilterUnreadOnly,
    ToggleGhNotificationFilterParticipatingOnly,
    ToggleGhNotificationFilterVisibilityPublic,
    ToggleGhNotificationFilterVisibilityPrivate,
    ToggleGhNotificationFilterPullRequestOpen,
    ToggleGhNotificationFilterPullRequestClosed,
    ToggleGhNotificationFilterPullRequestMerged,
    ToggleGhNotificationFilterReasonMentioned,
    ToggleGhNotificationFilterReasonReviewRequested,
}

impl CommandId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Nop => "nop",
            Self::Quit => "app.quit",
            Self::ForceRedraw => "app.redraw",
            Self::RotateTheme => "theme.rotate",
            Self::Authenticate => "login.authenticate",
            Self::MoveAuthenticationProviderPrev => "login.provider.prev",
            Self::MoveAuthenticationProviderNext => "login.provider.next",
            Self::MoveTabPrev => "tabs.prev",
            Self::MoveTabNext => "tabs.next",
            Self::MoveEntryPrev => "entries.prev",
            Self::MoveEntryNext => "entries.next",
            Self::MoveEntryFirst => "entries.first",
            Self::MoveEntryLast => "entries.last",
            Self::RefreshTimeline => "timeline.refresh",
            Self::OpenEntry => "entries.open",
            Self::BrowseEntry => "entries.browse",
            Self::MoveSubscribedFeedPrev => "feeds.prev",
            Self::MoveSubscribedFeedNext => "feeds.next",
            Self::MoveSubscribedFeedFirst => "feeds.first",
            Self::MoveSubscribedFeedLast => "feeds.last",
            Self::PromptFeedSubscription => "feeds.subscribe",
            Self::PromptFeedEdition => "feeds.edit",
            Self::PromptFeedUnsubscription => "feeds.unsubscribe",
            Self::ReloadSubscription => "feeds.reload",
            Self::OpenFeed => "feeds.open",
            Self::MoveFeedUnsubscriptionPopupSelectionPrev => "feeds.unsubscribe-popup.prev",
            Self::MoveFeedUnsubscriptionPopupSelectionNext => "feeds.unsubscribe-popup.next",
            Self::SelectFeedUnsubscriptionPopup => "feeds.unsubscribe-popup.select",
            Self::CancelFeedUnsubscriptionPopup => "feeds.unsubscribe-popup.cancel",
            Self::MoveFilterRequirementPrev => "filter.requirement.prev",
            Self::MoveFilterRequirementNext => "filter.requirement.next",
            Self::ActivateCategoryFiltering => "filter.category",
            Self::ActivateSearchFiltering => "filter.search",
            Self::DeactivateFiltering => "filter.close",
            Self::MoveGhNotificationPrev => "github-notifications.prev",
            Self::MoveGhNotificationNext => "github-notifications.next",
            Self::MoveGhNotificationFirst => "github-notifications.first",
            Self::MoveGhNotificationLast => "github-notifications.last",
            Self::OpenGhNotification => "github-notifications.open",
            Self::OpenGhNotificationAndMarkAsDone => "github-notifications.open-and-done",
            Self::ReloadGhNotifications => "github-notifications.reload",
            Self::MarkGhNotificationAsDone => "github-notifications.mark-done",
            Self::MarkAllGhNotificationsAsDone => "github-notifications.mark-all-done",
            Self::UnsubscribeGhThread => "github-notifications.unsubscribe-thread",
            Self::OpenGhNotificationFilter => "github-notifications.filter.open",
            Self::CloseGhNotificationFilter => "github-notifications.filter.close",
            Self::ToggleGhNotificationFilterUnreadOnly => {
                "github-notifications.filter.include-unread.toggle"
            }
            Self::ToggleGhNotificationFilterParticipatingOnly => {
                "github-notifications.filter.participating.toggle"
            }
            Self::ToggleGhNotificationFilterVisibilityPublic => {
                "github-notifications.filter.visibility-public.toggle"
            }
            Self::ToggleGhNotificationFilterVisibilityPrivate => {
                "github-notifications.filter.visibility-private.toggle"
            }
            Self::ToggleGhNotificationFilterPullRequestOpen => {
                "github-notifications.filter.pr-open.toggle"
            }
            Self::ToggleGhNotificationFilterPullRequestClosed => {
                "github-notifications.filter.pr-closed.toggle"
            }
            Self::ToggleGhNotificationFilterPullRequestMerged => {
                "github-notifications.filter.pr-merged.toggle"
            }
            Self::ToggleGhNotificationFilterReasonMentioned => {
                "github-notifications.filter.reason-mentioned.toggle"
            }
            Self::ToggleGhNotificationFilterReasonReviewRequested => {
                "github-notifications.filter.reason-review-requested.toggle"
            }
        }
    }
}

impl From<CommandId> for Command {
    fn from(command: CommandId) -> Self {
        match command {
            CommandId::Nop => Command::Nop,
            CommandId::Quit => Command::Shell(ShellCommand::Quit),
            CommandId::ForceRedraw => Command::Shell(ShellCommand::ForceRedraw),
            CommandId::RotateTheme => Command::Shell(ShellCommand::RotateTheme),
            CommandId::Authenticate => Command::Shell(ShellCommand::Authenticate),
            CommandId::MoveAuthenticationProviderPrev => {
                Command::Shell(ShellCommand::MoveAuthenticationProvider(Direction::Up))
            }
            CommandId::MoveAuthenticationProviderNext => {
                Command::Shell(ShellCommand::MoveAuthenticationProvider(Direction::Down))
            }
            CommandId::MoveTabPrev => {
                Command::Shell(ShellCommand::MoveTabSelection(Direction::Left))
            }
            CommandId::MoveTabNext => {
                Command::Shell(ShellCommand::MoveTabSelection(Direction::Right))
            }
            CommandId::MoveEntryPrev => Command::Feeds(FeedsCommand::MoveEntry(Direction::Up)),
            CommandId::MoveEntryNext => Command::Feeds(FeedsCommand::MoveEntry(Direction::Down)),
            CommandId::MoveEntryFirst => Command::Feeds(FeedsCommand::MoveEntryFirst),
            CommandId::MoveEntryLast => Command::Feeds(FeedsCommand::MoveEntryLast),
            CommandId::RefreshTimeline => Command::Feeds(FeedsCommand::RefreshTimeline),
            CommandId::OpenEntry => Command::Feeds(FeedsCommand::OpenEntry),
            CommandId::BrowseEntry => Command::Feeds(FeedsCommand::BrowseEntry),
            CommandId::MoveSubscribedFeedPrev => {
                Command::Feeds(FeedsCommand::MoveSubscribedFeed(Direction::Up))
            }
            CommandId::MoveSubscribedFeedNext => {
                Command::Feeds(FeedsCommand::MoveSubscribedFeed(Direction::Down))
            }
            CommandId::MoveSubscribedFeedFirst => {
                Command::Feeds(FeedsCommand::MoveSubscribedFeedFirst)
            }
            CommandId::MoveSubscribedFeedLast => {
                Command::Feeds(FeedsCommand::MoveSubscribedFeedLast)
            }
            CommandId::PromptFeedSubscription => {
                Command::Feeds(FeedsCommand::PromptFeedSubscription)
            }
            CommandId::PromptFeedEdition => Command::Feeds(FeedsCommand::PromptFeedEdition),
            CommandId::PromptFeedUnsubscription => {
                Command::Feeds(FeedsCommand::PromptFeedUnsubscription)
            }
            CommandId::ReloadSubscription => Command::Feeds(FeedsCommand::ReloadSubscription),
            CommandId::OpenFeed => Command::Feeds(FeedsCommand::OpenFeed),
            CommandId::MoveFeedUnsubscriptionPopupSelectionPrev => Command::Feeds(
                FeedsCommand::MoveFeedUnsubscriptionPopupSelection(Direction::Left),
            ),
            CommandId::MoveFeedUnsubscriptionPopupSelectionNext => Command::Feeds(
                FeedsCommand::MoveFeedUnsubscriptionPopupSelection(Direction::Right),
            ),
            CommandId::SelectFeedUnsubscriptionPopup => {
                Command::Feeds(FeedsCommand::SelectFeedUnsubscriptionPopup)
            }
            CommandId::CancelFeedUnsubscriptionPopup => {
                Command::Feeds(FeedsCommand::CancelFeedUnsubscriptionPopup)
            }
            CommandId::MoveFilterRequirementPrev => {
                Command::Filter(FilterCommand::MoveFilterRequirement(Direction::Left))
            }
            CommandId::MoveFilterRequirementNext => {
                Command::Filter(FilterCommand::MoveFilterRequirement(Direction::Right))
            }
            CommandId::ActivateCategoryFiltering => {
                Command::Filter(FilterCommand::ActivateCategoryFiltering)
            }
            CommandId::ActivateSearchFiltering => {
                Command::Filter(FilterCommand::ActivateSearchFiltering)
            }
            CommandId::DeactivateFiltering => Command::Filter(FilterCommand::DeactivateFiltering),
            CommandId::MoveGhNotificationPrev => {
                Command::Gh(GhCommand::MoveNotification(Direction::Up))
            }
            CommandId::MoveGhNotificationNext => {
                Command::Gh(GhCommand::MoveNotification(Direction::Down))
            }
            CommandId::MoveGhNotificationFirst => Command::Gh(GhCommand::MoveNotificationFirst),
            CommandId::MoveGhNotificationLast => Command::Gh(GhCommand::MoveNotificationLast),
            CommandId::OpenGhNotification => Command::Gh(GhCommand::OpenNotification),
            CommandId::OpenGhNotificationAndMarkAsDone => {
                Command::Gh(GhCommand::OpenNotificationAndMarkAsDone)
            }
            CommandId::ReloadGhNotifications => Command::Gh(GhCommand::ReloadNotifications),
            CommandId::MarkGhNotificationAsDone => Command::Gh(GhCommand::MarkNotificationAsDone),
            CommandId::MarkAllGhNotificationsAsDone => {
                Command::Gh(GhCommand::MarkAllNotificationsAsDone)
            }
            CommandId::UnsubscribeGhThread => Command::Gh(GhCommand::UnsubscribeThread),
            CommandId::OpenGhNotificationFilter => Command::Gh(GhCommand::OpenNotificationFilter),
            CommandId::CloseGhNotificationFilter => Command::Gh(GhCommand::CloseNotificationFilter),
            CommandId::ToggleGhNotificationFilterUnreadOnly => Command::Gh(
                GhCommand::ToggleNotificationFilter(GhNotificationFilterOption::UnreadOnly),
            ),
            CommandId::ToggleGhNotificationFilterParticipatingOnly => Command::Gh(
                GhCommand::ToggleNotificationFilter(GhNotificationFilterOption::ParticipatingOnly),
            ),
            CommandId::ToggleGhNotificationFilterVisibilityPublic => {
                Command::Gh(GhCommand::ToggleNotificationFilter(
                    GhNotificationFilterOption::Visibility(RepoVisibility::Public),
                ))
            }
            CommandId::ToggleGhNotificationFilterVisibilityPrivate => {
                Command::Gh(GhCommand::ToggleNotificationFilter(
                    GhNotificationFilterOption::Visibility(RepoVisibility::Private),
                ))
            }
            CommandId::ToggleGhNotificationFilterPullRequestOpen => {
                Command::Gh(GhCommand::ToggleNotificationFilter(
                    GhNotificationFilterOption::PullRequestState(PullRequestState::Open),
                ))
            }
            CommandId::ToggleGhNotificationFilterPullRequestClosed => {
                Command::Gh(GhCommand::ToggleNotificationFilter(
                    GhNotificationFilterOption::PullRequestState(PullRequestState::Closed),
                ))
            }
            CommandId::ToggleGhNotificationFilterPullRequestMerged => {
                Command::Gh(GhCommand::ToggleNotificationFilter(
                    GhNotificationFilterOption::PullRequestState(PullRequestState::Merged),
                ))
            }
            CommandId::ToggleGhNotificationFilterReasonMentioned => {
                Command::Gh(GhCommand::ToggleNotificationFilter(
                    GhNotificationFilterOption::Reason(Reason::Mention),
                ))
            }
            CommandId::ToggleGhNotificationFilterReasonReviewRequested => {
                Command::Gh(GhCommand::ToggleNotificationFilter(
                    GhNotificationFilterOption::Reason(Reason::ReviewRequested),
                ))
            }
        }
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CommandId {
    type Err = KeymapError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        CommandRegistry.command_id(value)
    }
}

impl<'de> Deserialize<'de> for CommandId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Metadata used to parse and validate a command id.
pub(crate) struct CommandSpec {
    pub(crate) id: CommandId,
    /// Additional names accepted in config besides the canonical id.
    pub(crate) aliases: &'static [&'static str],
    pub(crate) typable: Option<&'static str>,
    pub(crate) layers: &'static [Layer],
}

impl CommandSpec {
    fn is_allowed_in_layer(&self, layer: Layer) -> bool {
        self.id == CommandId::Nop || self.layers.contains(&layer)
    }
}

/// Catalog of command ids available to keymap configuration.
pub(crate) struct CommandRegistry;

impl CommandRegistry {
    #[expect(clippy::unused_self)]
    pub(crate) fn spec(&self, id: CommandId) -> &'static CommandSpec {
        COMMAND_SPECS
            .iter()
            .find(|spec| spec.id == id)
            .expect("registered command")
    }

    #[expect(clippy::unused_self)]
    pub(crate) fn command_id(&self, value: &str) -> Result<CommandId, KeymapError> {
        COMMAND_SPECS
            .iter()
            .find(|spec| {
                spec.id.as_str() == value
                    || spec.aliases.contains(&value)
                    || spec.typable == Some(value)
            })
            .map(|spec| spec.id)
            .ok_or_else(|| KeymapError::UnknownCommand(value.to_owned()))
    }

    pub(super) fn validate_binding(
        &self,
        layer: Layer,
        binding: &KeyBinding,
    ) -> Result<(), KeymapError> {
        let spec = self.spec(binding.command);
        if spec.is_allowed_in_layer(layer) {
            Ok(())
        } else {
            Err(KeymapError::CommandNotAllowed {
                layer,
                command: binding.command,
            })
        }
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self
    }
}

macro_rules! specs {
    (
        $(
            $id:ident {
                aliases: [$($alias:literal),* $(,)?],
                typable: $typable:expr,
                layers: [$($layer:ident),* $(,)?],
            }
        ),* $(,)?
    ) => {
        &[
            $(
                CommandSpec {
                    id: CommandId::$id,
                    aliases: &[$($alias),*],
                    typable: $typable,
                    layers: &[$(Layer::$layer),*],
                },
            )*
        ]
    };
}

const COMMAND_SPECS: &[CommandSpec] = specs![
    Nop {
        aliases: ["no_op"],
        typable: None,
        layers: [],
    },
    Quit {
        aliases: ["quit"],
        typable: Some(":quit"),
        layers: [App, Global],
    },
    ForceRedraw {
        aliases: ["force_redraw"],
        typable: Some(":redraw"),
        layers: [App],
    },
    RotateTheme {
        aliases: ["rotate_theme"],
        typable: None,
        layers: [Global],
    },
    Authenticate {
        aliases: ["authenticate"],
        typable: None,
        layers: [Login],
    },
    MoveAuthenticationProviderPrev {
        aliases: ["move_up_authentication_provider"],
        typable: None,
        layers: [Login],
    },
    MoveAuthenticationProviderNext {
        aliases: ["move_down_authentication_provider"],
        typable: None,
        layers: [Login],
    },
    MoveTabPrev {
        aliases: ["move_left_tab_selection"],
        typable: None,
        layers: [Tabs],
    },
    MoveTabNext {
        aliases: ["move_right_tab_selection"],
        typable: None,
        layers: [Tabs],
    },
    MoveEntryPrev {
        aliases: ["move_up_entry"],
        typable: None,
        layers: [Entries],
    },
    MoveEntryNext {
        aliases: ["move_down_entry"],
        typable: None,
        layers: [Entries],
    },
    MoveEntryFirst {
        aliases: ["move_entry_first"],
        typable: None,
        layers: [Entries],
    },
    MoveEntryLast {
        aliases: ["move_entry_last"],
        typable: None,
        layers: [Entries],
    },
    RefreshTimeline {
        aliases: [],
        typable: Some(":refresh-timeline"),
        layers: [Entries],
    },
    OpenEntry {
        aliases: ["open_entry"],
        typable: Some(":open-entry"),
        layers: [Entries],
    },
    BrowseEntry {
        aliases: ["browse_entry"],
        typable: None,
        layers: [Entries],
    },
    MoveSubscribedFeedPrev {
        aliases: ["move_up_subscribed_feed"],
        typable: None,
        layers: [Feeds],
    },
    MoveSubscribedFeedNext {
        aliases: ["move_down_subscribed_feed"],
        typable: None,
        layers: [Feeds],
    },
    MoveSubscribedFeedFirst {
        aliases: ["move_subscribed_feed_first"],
        typable: None,
        layers: [Feeds],
    },
    MoveSubscribedFeedLast {
        aliases: ["move_subscribed_feed_last"],
        typable: None,
        layers: [Feeds],
    },
    PromptFeedSubscription {
        aliases: ["prompt_feed_subscription"],
        typable: None,
        layers: [Feeds],
    },
    PromptFeedEdition {
        aliases: ["prompt_feed_edition"],
        typable: None,
        layers: [Feeds],
    },
    PromptFeedUnsubscription {
        aliases: ["prompt_feed_unsubscription"],
        typable: None,
        layers: [Feeds],
    },
    ReloadSubscription {
        aliases: ["reload_subscription"],
        typable: Some(":reload-subscription"),
        layers: [Feeds],
    },
    OpenFeed {
        aliases: ["open_feed"],
        typable: None,
        layers: [Feeds],
    },
    MoveFeedUnsubscriptionPopupSelectionPrev {
        aliases: ["move_feed_unsubscription_popup_selection_left"],
        typable: None,
        layers: [UnsubscribePopup],
    },
    MoveFeedUnsubscriptionPopupSelectionNext {
        aliases: ["move_feed_unsubscription_popup_selection_right"],
        typable: None,
        layers: [UnsubscribePopup],
    },
    SelectFeedUnsubscriptionPopup {
        aliases: ["select_feed_unsubscription_popup"],
        typable: None,
        layers: [UnsubscribePopup],
    },
    CancelFeedUnsubscriptionPopup {
        aliases: ["cancel_feed_unsubscription_popup"],
        typable: None,
        layers: [UnsubscribePopup],
    },
    MoveFilterRequirementPrev {
        aliases: ["move_filter_requirement_left"],
        typable: None,
        layers: [Filter],
    },
    MoveFilterRequirementNext {
        aliases: ["move_filter_requirement_right"],
        typable: None,
        layers: [Filter],
    },
    ActivateCategoryFiltering {
        aliases: ["activate_category_filtering"],
        typable: None,
        layers: [Filter],
    },
    ActivateSearchFiltering {
        aliases: ["activate_search_filtering"],
        typable: None,
        layers: [Filter],
    },
    DeactivateFiltering {
        aliases: ["deactivate_filtering"],
        typable: None,
        layers: [Filter],
    },
    MoveGhNotificationPrev {
        aliases: ["move_up_gh_notification"],
        typable: None,
        layers: [GhNotifications],
    },
    MoveGhNotificationNext {
        aliases: ["move_down_gh_notification"],
        typable: None,
        layers: [GhNotifications],
    },
    MoveGhNotificationFirst {
        aliases: ["move_gh_notification_first"],
        typable: None,
        layers: [GhNotifications],
    },
    MoveGhNotificationLast {
        aliases: ["move_gh_notification_last"],
        typable: None,
        layers: [GhNotifications],
    },
    OpenGhNotification {
        aliases: ["open_gh_notification"],
        typable: None,
        layers: [GhNotifications],
    },
    OpenGhNotificationAndMarkAsDone {
        aliases: ["open_gh_notification_with_done"],
        typable: None,
        layers: [GhNotifications],
    },
    ReloadGhNotifications {
        aliases: ["reload_gh_notifications"],
        typable: None,
        layers: [GhNotifications],
    },
    MarkGhNotificationAsDone {
        aliases: ["mark_gh_notification_as_done"],
        typable: None,
        layers: [GhNotifications],
    },
    MarkAllGhNotificationsAsDone {
        aliases: ["mark_gh_notification_as_done_all"],
        typable: None,
        layers: [GhNotifications],
    },
    UnsubscribeGhThread {
        aliases: ["unsubscribe_gh_thread"],
        typable: None,
        layers: [GhNotifications],
    },
    OpenGhNotificationFilter {
        aliases: ["open_gh_notification_filter_popup"],
        typable: None,
        layers: [GhNotifications],
    },
    CloseGhNotificationFilter {
        aliases: ["close_gh_notification_filter_popup"],
        typable: None,
        layers: [GhNotificationFilterPopup],
    },
    ToggleGhNotificationFilterUnreadOnly {
        aliases: ["toggle_gh_notification_filter_popup_include_unread"],
        typable: None,
        layers: [GhNotificationFilterPopup],
    },
    ToggleGhNotificationFilterParticipatingOnly {
        aliases: ["toggle_gh_notification_filter_popup_participating"],
        typable: None,
        layers: [GhNotificationFilterPopup],
    },
    ToggleGhNotificationFilterVisibilityPublic {
        aliases: ["toggle_gh_notification_filter_popup_visibility_public"],
        typable: None,
        layers: [GhNotificationFilterPopup],
    },
    ToggleGhNotificationFilterVisibilityPrivate {
        aliases: ["toggle_gh_notification_filter_popup_visibility_private"],
        typable: None,
        layers: [GhNotificationFilterPopup],
    },
    ToggleGhNotificationFilterPullRequestOpen {
        aliases: ["toggle_gh_notification_filter_popup_pr_open"],
        typable: None,
        layers: [GhNotificationFilterPopup],
    },
    ToggleGhNotificationFilterPullRequestClosed {
        aliases: ["toggle_gh_notification_filter_popup_pr_closed"],
        typable: None,
        layers: [GhNotificationFilterPopup],
    },
    ToggleGhNotificationFilterPullRequestMerged {
        aliases: ["toggle_gh_notification_filter_popup_pr_merged"],
        typable: None,
        layers: [GhNotificationFilterPopup],
    },
    ToggleGhNotificationFilterReasonMentioned {
        aliases: ["toggle_gh_notification_filter_popup_reason_mentioned"],
        typable: None,
        layers: [GhNotificationFilterPopup],
    },
    ToggleGhNotificationFilterReasonReviewRequested {
        aliases: ["toggle_gh_notification_filter_popup_reason_review"],
        typable: None,
        layers: [GhNotificationFilterPopup],
    },
];
