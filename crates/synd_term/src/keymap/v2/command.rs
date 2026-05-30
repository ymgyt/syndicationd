use std::{fmt, str::FromStr};

use crate::command::Command;
use serde::Deserialize;

use super::{KeyBinding, KeymapError, Layer};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CommandId {
    #[default]
    NoOp,
    Quit,
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
    ReloadEntries,
    OpenEntry,
    BrowseEntry,
    MoveSubscribedFeedPrev,
    MoveSubscribedFeedNext,
    MoveSubscribedFeedFirst,
    MoveSubscribedFeedLast,
    PromptFeedSubscription,
    PromptFeedEdition,
    PromptFeedUnsubscription,
    RefreshSelectedFeed,
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
    MoveGithubNotificationPrev,
    MoveGithubNotificationNext,
    MoveGithubNotificationFirst,
    MoveGithubNotificationLast,
    OpenGithubNotification,
    OpenGithubNotificationWithDone,
    ReloadGithubNotifications,
    MarkGithubNotificationAsDone,
    MarkGithubNotificationAsDoneAll,
    UnsubscribeGithubThread,
    OpenGithubNotificationFilterPopup,
    CloseGithubNotificationFilterPopup,
    ToggleGithubNotificationFilterPopupIncludeUnread,
    ToggleGithubNotificationFilterPopupParticipating,
    ToggleGithubNotificationFilterPopupVisibilityPublic,
    ToggleGithubNotificationFilterPopupVisibilityPrivate,
    ToggleGithubNotificationFilterPopupPullRequestOpen,
    ToggleGithubNotificationFilterPopupPullRequestClosed,
    ToggleGithubNotificationFilterPopupPullRequestMerged,
    ToggleGithubNotificationFilterPopupReasonMentioned,
    ToggleGithubNotificationFilterPopupReasonReviewRequested,
}

impl CommandId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoOp => "no_op",
            Self::Quit => "app.quit",
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
            Self::ReloadEntries => "entries.reload",
            Self::OpenEntry => "entries.open",
            Self::BrowseEntry => "entries.browse",
            Self::MoveSubscribedFeedPrev => "feeds.prev",
            Self::MoveSubscribedFeedNext => "feeds.next",
            Self::MoveSubscribedFeedFirst => "feeds.first",
            Self::MoveSubscribedFeedLast => "feeds.last",
            Self::PromptFeedSubscription => "feeds.subscribe",
            Self::PromptFeedEdition => "feeds.edit",
            Self::PromptFeedUnsubscription => "feeds.unsubscribe",
            Self::RefreshSelectedFeed => "feeds.refresh-selected",
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
            Self::MoveGithubNotificationPrev => "github-notifications.prev",
            Self::MoveGithubNotificationNext => "github-notifications.next",
            Self::MoveGithubNotificationFirst => "github-notifications.first",
            Self::MoveGithubNotificationLast => "github-notifications.last",
            Self::OpenGithubNotification => "github-notifications.open",
            Self::OpenGithubNotificationWithDone => "github-notifications.open-and-done",
            Self::ReloadGithubNotifications => "github-notifications.reload",
            Self::MarkGithubNotificationAsDone => "github-notifications.mark-done",
            Self::MarkGithubNotificationAsDoneAll => "github-notifications.mark-all-done",
            Self::UnsubscribeGithubThread => "github-notifications.unsubscribe-thread",
            Self::OpenGithubNotificationFilterPopup => "github-notifications.filter.open",
            Self::CloseGithubNotificationFilterPopup => "github-notifications.filter.close",
            Self::ToggleGithubNotificationFilterPopupIncludeUnread => {
                "github-notifications.filter.include-unread.toggle"
            }
            Self::ToggleGithubNotificationFilterPopupParticipating => {
                "github-notifications.filter.participating.toggle"
            }
            Self::ToggleGithubNotificationFilterPopupVisibilityPublic => {
                "github-notifications.filter.visibility-public.toggle"
            }
            Self::ToggleGithubNotificationFilterPopupVisibilityPrivate => {
                "github-notifications.filter.visibility-private.toggle"
            }
            Self::ToggleGithubNotificationFilterPopupPullRequestOpen => {
                "github-notifications.filter.pr-open.toggle"
            }
            Self::ToggleGithubNotificationFilterPopupPullRequestClosed => {
                "github-notifications.filter.pr-closed.toggle"
            }
            Self::ToggleGithubNotificationFilterPopupPullRequestMerged => {
                "github-notifications.filter.pr-merged.toggle"
            }
            Self::ToggleGithubNotificationFilterPopupReasonMentioned => {
                "github-notifications.filter.reason-mentioned.toggle"
            }
            Self::ToggleGithubNotificationFilterPopupReasonReviewRequested => {
                "github-notifications.filter.reason-review-requested.toggle"
            }
        }
    }

    pub(super) fn build(self) -> Option<Command> {
        match self {
            Self::NoOp => None,
            Self::Quit => Some(Command::quit()),
            Self::RotateTheme => Some(Command::rotate_theme()),
            Self::Authenticate => Some(Command::authenticate()),
            Self::MoveAuthenticationProviderPrev => {
                Some(Command::move_up_authentication_provider())
            }
            Self::MoveAuthenticationProviderNext => {
                Some(Command::move_down_authentication_provider())
            }
            Self::MoveTabPrev => Some(Command::move_left_tab_selection()),
            Self::MoveTabNext => Some(Command::move_right_tab_selection()),
            Self::MoveEntryPrev => Some(Command::move_up_entry()),
            Self::MoveEntryNext => Some(Command::move_down_entry()),
            Self::MoveEntryFirst => Some(Command::move_entry_first()),
            Self::MoveEntryLast => Some(Command::move_entry_last()),
            Self::ReloadEntries => Some(Command::reload_entries()),
            Self::OpenEntry => Some(Command::open_entry()),
            Self::BrowseEntry => Some(Command::browse_entry()),
            Self::MoveSubscribedFeedPrev => Some(Command::move_up_subscribed_feed()),
            Self::MoveSubscribedFeedNext => Some(Command::move_down_subscribed_feed()),
            Self::MoveSubscribedFeedFirst => Some(Command::move_subscribed_feed_first()),
            Self::MoveSubscribedFeedLast => Some(Command::move_subscribed_feed_last()),
            Self::PromptFeedSubscription => Some(Command::prompt_feed_subscription()),
            Self::PromptFeedEdition => Some(Command::prompt_feed_edition()),
            Self::PromptFeedUnsubscription => Some(Command::prompt_feed_unsubscription()),
            Self::RefreshSelectedFeed => Some(Command::refresh_selected_feed()),
            Self::ReloadSubscription => Some(Command::reload_subscription()),
            Self::OpenFeed => Some(Command::open_feed()),
            Self::MoveFeedUnsubscriptionPopupSelectionPrev => {
                Some(Command::move_feed_unsubscription_popup_selection_left())
            }
            Self::MoveFeedUnsubscriptionPopupSelectionNext => {
                Some(Command::move_feed_unsubscription_popup_selection_right())
            }
            Self::SelectFeedUnsubscriptionPopup => {
                Some(Command::select_feed_unsubscription_popup())
            }
            Self::CancelFeedUnsubscriptionPopup => {
                Some(Command::cancel_feed_unsubscription_popup())
            }
            Self::MoveFilterRequirementPrev => Some(Command::move_filter_requirement_left()),
            Self::MoveFilterRequirementNext => Some(Command::move_filter_requirement_right()),
            Self::ActivateCategoryFiltering => Some(Command::activate_category_filtering()),
            Self::ActivateSearchFiltering => Some(Command::activate_search_filtering()),
            Self::DeactivateFiltering => Some(Command::deactivate_filtering()),
            Self::MoveGithubNotificationPrev => Some(Command::move_up_gh_notification()),
            Self::MoveGithubNotificationNext => Some(Command::move_down_gh_notification()),
            Self::MoveGithubNotificationFirst => Some(Command::move_gh_notification_first()),
            Self::MoveGithubNotificationLast => Some(Command::move_gh_notification_last()),
            Self::OpenGithubNotification => Some(Command::open_gh_notification()),
            Self::OpenGithubNotificationWithDone => Some(Command::open_gh_notification_with_done()),
            Self::ReloadGithubNotifications => Some(Command::reload_gh_notifications()),
            Self::MarkGithubNotificationAsDone => Some(Command::mark_gh_notification_as_done()),
            Self::MarkGithubNotificationAsDoneAll => {
                Some(Command::mark_gh_notification_as_done_all())
            }
            Self::UnsubscribeGithubThread => Some(Command::unsubscribe_gh_thread()),
            Self::OpenGithubNotificationFilterPopup => {
                Some(Command::open_gh_notification_filter_popup())
            }
            Self::CloseGithubNotificationFilterPopup => {
                Some(Command::close_gh_notification_filter_popup())
            }
            Self::ToggleGithubNotificationFilterPopupIncludeUnread => {
                Some(Command::toggle_gh_notification_filter_popup_include_unread())
            }
            Self::ToggleGithubNotificationFilterPopupParticipating => {
                Some(Command::toggle_gh_notification_filter_popup_participating())
            }
            Self::ToggleGithubNotificationFilterPopupVisibilityPublic => {
                Some(Command::toggle_gh_notification_filter_popup_visibility_public())
            }
            Self::ToggleGithubNotificationFilterPopupVisibilityPrivate => {
                Some(Command::toggle_gh_notification_filter_popup_visibility_private())
            }
            Self::ToggleGithubNotificationFilterPopupPullRequestOpen => {
                Some(Command::toggle_gh_notification_filter_popup_pr_open())
            }
            Self::ToggleGithubNotificationFilterPopupPullRequestClosed => {
                Some(Command::toggle_gh_notification_filter_popup_pr_closed())
            }
            Self::ToggleGithubNotificationFilterPopupPullRequestMerged => {
                Some(Command::toggle_gh_notification_filter_popup_pr_merged())
            }
            Self::ToggleGithubNotificationFilterPopupReasonMentioned => {
                Some(Command::toggle_gh_notification_filter_popup_reason_mentioned())
            }
            Self::ToggleGithubNotificationFilterPopupReasonReviewRequested => {
                Some(Command::toggle_gh_notification_filter_popup_reason_review())
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

pub(crate) struct CommandSpec {
    pub(crate) id: CommandId,
    pub(crate) aliases: &'static [&'static str],
    pub(crate) typable: Option<&'static str>,
    pub(crate) desc: &'static str,
    pub(crate) layers: &'static [Layer],
}

impl CommandSpec {
    fn allows_layer(&self, layer: Layer) -> bool {
        self.id == CommandId::NoOp || self.layers.contains(&layer)
    }
}

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
        if spec.allows_layer(layer) {
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
                desc: $desc:literal,
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
                    desc: $desc,
                    layers: &[$(Layer::$layer),*],
                },
            )*
        ]
    };
}

const COMMAND_SPECS: &[CommandSpec] = specs![
    NoOp {
        aliases: [],
        typable: None,
        desc: "Do nothing",
        layers: [],
    },
    Quit {
        aliases: ["quit"],
        typable: Some(":quit"),
        desc: "Quit app",
        layers: [App, Global],
    },
    RotateTheme {
        aliases: ["rotate_theme"],
        typable: None,
        desc: "Rotate theme",
        layers: [Global],
    },
    Authenticate {
        aliases: ["authenticate"],
        typable: None,
        desc: "Authenticate",
        layers: [Login],
    },
    MoveAuthenticationProviderPrev {
        aliases: ["move_up_authentication_provider"],
        typable: None,
        desc: "Previous authentication provider",
        layers: [Login],
    },
    MoveAuthenticationProviderNext {
        aliases: ["move_down_authentication_provider"],
        typable: None,
        desc: "Next authentication provider",
        layers: [Login],
    },
    MoveTabPrev {
        aliases: ["move_left_tab_selection"],
        typable: None,
        desc: "Previous tab",
        layers: [Tabs],
    },
    MoveTabNext {
        aliases: ["move_right_tab_selection"],
        typable: None,
        desc: "Next tab",
        layers: [Tabs],
    },
    MoveEntryPrev {
        aliases: ["move_up_entry"],
        typable: None,
        desc: "Previous entry",
        layers: [Entries],
    },
    MoveEntryNext {
        aliases: ["move_down_entry"],
        typable: None,
        desc: "Next entry",
        layers: [Entries],
    },
    MoveEntryFirst {
        aliases: ["move_entry_first"],
        typable: None,
        desc: "Go to first entry",
        layers: [Entries],
    },
    MoveEntryLast {
        aliases: ["move_entry_last"],
        typable: None,
        desc: "Go to last entry",
        layers: [Entries],
    },
    ReloadEntries {
        aliases: ["reload_entries"],
        typable: Some(":reload-entries"),
        desc: "Reload entries",
        layers: [Entries],
    },
    OpenEntry {
        aliases: ["open_entry"],
        typable: Some(":open-entry"),
        desc: "Open entry",
        layers: [Entries],
    },
    BrowseEntry {
        aliases: ["browse_entry"],
        typable: None,
        desc: "Browse entry",
        layers: [Entries],
    },
    MoveSubscribedFeedPrev {
        aliases: ["move_up_subscribed_feed"],
        typable: None,
        desc: "Previous feed",
        layers: [Feeds],
    },
    MoveSubscribedFeedNext {
        aliases: ["move_down_subscribed_feed"],
        typable: None,
        desc: "Next feed",
        layers: [Feeds],
    },
    MoveSubscribedFeedFirst {
        aliases: ["move_subscribed_feed_first"],
        typable: None,
        desc: "Go to first feed",
        layers: [Feeds],
    },
    MoveSubscribedFeedLast {
        aliases: ["move_subscribed_feed_last"],
        typable: None,
        desc: "Go to last feed",
        layers: [Feeds],
    },
    PromptFeedSubscription {
        aliases: ["prompt_feed_subscription"],
        typable: None,
        desc: "Add feed subscription",
        layers: [Feeds],
    },
    PromptFeedEdition {
        aliases: ["prompt_feed_edition"],
        typable: None,
        desc: "Edit feed subscription",
        layers: [Feeds],
    },
    PromptFeedUnsubscription {
        aliases: ["prompt_feed_unsubscription"],
        typable: None,
        desc: "Delete feed subscription",
        layers: [Feeds],
    },
    RefreshSelectedFeed {
        aliases: ["refresh_selected_feed"],
        typable: Some(":refresh-selected-feed"),
        desc: "Refresh selected feed",
        layers: [Feeds],
    },
    ReloadSubscription {
        aliases: ["reload_subscription"],
        typable: Some(":reload-subscription"),
        desc: "Reload subscriptions",
        layers: [Feeds],
    },
    OpenFeed {
        aliases: ["open_feed"],
        typable: None,
        desc: "Open feed",
        layers: [Feeds],
    },
    MoveFeedUnsubscriptionPopupSelectionPrev {
        aliases: ["move_feed_unsubscription_popup_selection_left"],
        typable: None,
        desc: "Previous popup selection",
        layers: [UnsubscribePopup],
    },
    MoveFeedUnsubscriptionPopupSelectionNext {
        aliases: ["move_feed_unsubscription_popup_selection_right"],
        typable: None,
        desc: "Next popup selection",
        layers: [UnsubscribePopup],
    },
    SelectFeedUnsubscriptionPopup {
        aliases: ["select_feed_unsubscription_popup"],
        typable: None,
        desc: "Select popup item",
        layers: [UnsubscribePopup],
    },
    CancelFeedUnsubscriptionPopup {
        aliases: ["cancel_feed_unsubscription_popup"],
        typable: None,
        desc: "Cancel popup",
        layers: [UnsubscribePopup],
    },
    MoveFilterRequirementPrev {
        aliases: ["move_filter_requirement_left"],
        typable: None,
        desc: "Previous requirement filter",
        layers: [Filter],
    },
    MoveFilterRequirementNext {
        aliases: ["move_filter_requirement_right"],
        typable: None,
        desc: "Next requirement filter",
        layers: [Filter],
    },
    ActivateCategoryFiltering {
        aliases: ["activate_category_filtering"],
        typable: None,
        desc: "Activate category filter",
        layers: [Filter],
    },
    ActivateSearchFiltering {
        aliases: ["activate_search_filtering"],
        typable: None,
        desc: "Activate search filter",
        layers: [Filter],
    },
    DeactivateFiltering {
        aliases: ["deactivate_filtering"],
        typable: None,
        desc: "Deactivate filter",
        layers: [Filter],
    },
    MoveGithubNotificationPrev {
        aliases: ["move_up_gh_notification"],
        typable: None,
        desc: "Previous GitHub notification",
        layers: [GithubNotifications],
    },
    MoveGithubNotificationNext {
        aliases: ["move_down_gh_notification"],
        typable: None,
        desc: "Next GitHub notification",
        layers: [GithubNotifications],
    },
    MoveGithubNotificationFirst {
        aliases: ["move_gh_notification_first"],
        typable: None,
        desc: "Go to first GitHub notification",
        layers: [GithubNotifications],
    },
    MoveGithubNotificationLast {
        aliases: ["move_gh_notification_last"],
        typable: None,
        desc: "Go to last GitHub notification",
        layers: [GithubNotifications],
    },
    OpenGithubNotification {
        aliases: ["open_gh_notification"],
        typable: None,
        desc: "Open GitHub notification",
        layers: [GithubNotifications],
    },
    OpenGithubNotificationWithDone {
        aliases: ["open_gh_notification_with_done"],
        typable: None,
        desc: "Open GitHub notification and mark as done",
        layers: [GithubNotifications],
    },
    ReloadGithubNotifications {
        aliases: ["reload_gh_notifications"],
        typable: None,
        desc: "Reload GitHub notifications",
        layers: [GithubNotifications],
    },
    MarkGithubNotificationAsDone {
        aliases: ["mark_gh_notification_as_done"],
        typable: None,
        desc: "Mark GitHub notification as done",
        layers: [GithubNotifications],
    },
    MarkGithubNotificationAsDoneAll {
        aliases: ["mark_gh_notification_as_done_all"],
        typable: None,
        desc: "Mark all GitHub notifications as done",
        layers: [GithubNotifications],
    },
    UnsubscribeGithubThread {
        aliases: ["unsubscribe_gh_thread"],
        typable: None,
        desc: "Unsubscribe GitHub thread",
        layers: [GithubNotifications],
    },
    OpenGithubNotificationFilterPopup {
        aliases: ["open_gh_notification_filter_popup"],
        typable: None,
        desc: "Open GitHub notification filter popup",
        layers: [GithubNotifications],
    },
    CloseGithubNotificationFilterPopup {
        aliases: ["close_gh_notification_filter_popup"],
        typable: None,
        desc: "Close GitHub notification filter popup",
        layers: [GithubNotificationFilterPopup],
    },
    ToggleGithubNotificationFilterPopupIncludeUnread {
        aliases: ["toggle_gh_notification_filter_popup_include_unread"],
        typable: None,
        desc: "Toggle unread filter",
        layers: [GithubNotificationFilterPopup],
    },
    ToggleGithubNotificationFilterPopupParticipating {
        aliases: ["toggle_gh_notification_filter_popup_participating"],
        typable: None,
        desc: "Toggle participating filter",
        layers: [GithubNotificationFilterPopup],
    },
    ToggleGithubNotificationFilterPopupVisibilityPublic {
        aliases: ["toggle_gh_notification_filter_popup_visibility_public"],
        typable: None,
        desc: "Toggle public repository filter",
        layers: [GithubNotificationFilterPopup],
    },
    ToggleGithubNotificationFilterPopupVisibilityPrivate {
        aliases: ["toggle_gh_notification_filter_popup_visibility_private"],
        typable: None,
        desc: "Toggle private repository filter",
        layers: [GithubNotificationFilterPopup],
    },
    ToggleGithubNotificationFilterPopupPullRequestOpen {
        aliases: ["toggle_gh_notification_filter_popup_pr_open"],
        typable: None,
        desc: "Toggle open pull request filter",
        layers: [GithubNotificationFilterPopup],
    },
    ToggleGithubNotificationFilterPopupPullRequestClosed {
        aliases: ["toggle_gh_notification_filter_popup_pr_closed"],
        typable: None,
        desc: "Toggle closed pull request filter",
        layers: [GithubNotificationFilterPopup],
    },
    ToggleGithubNotificationFilterPopupPullRequestMerged {
        aliases: ["toggle_gh_notification_filter_popup_pr_merged"],
        typable: None,
        desc: "Toggle merged pull request filter",
        layers: [GithubNotificationFilterPopup],
    },
    ToggleGithubNotificationFilterPopupReasonMentioned {
        aliases: ["toggle_gh_notification_filter_popup_reason_mentioned"],
        typable: None,
        desc: "Toggle mentioned reason filter",
        layers: [GithubNotificationFilterPopup],
    },
    ToggleGithubNotificationFilterPopupReasonReviewRequested {
        aliases: ["toggle_gh_notification_filter_popup_reason_review"],
        typable: None,
        desc: "Toggle review requested reason filter",
        layers: [GithubNotificationFilterPopup],
    },
];
