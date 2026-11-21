use super::{CommandId, KeymapConfig, Layer};

#[expect(clippy::too_many_lines)]
pub(super) fn default_keymap_config() -> KeymapConfig {
    let mut config = KeymapConfig::new();

    macro_rules! bind {
        ($layer:expr, [$($key:literal),+], $command:expr, $desc:literal) => {
            config
                .add($layer, [$($key),+], $command, Some($desc))
                .expect("valid default keymap binding");
        };
    }

    bind!(Layer::App, ["C-c"], CommandId::Quit, "Quit app");
    bind!(Layer::App, ["S-r"], CommandId::ForceRedraw, "Redraw screen");

    bind!(Layer::Global, ["q"], CommandId::Quit, "Quit app");
    bind!(
        Layer::Global,
        ["S-t"],
        CommandId::RotateTheme,
        "Rotate theme"
    );

    bind!(
        Layer::Login,
        ["enter"],
        CommandId::Authenticate,
        "Authenticate"
    );
    bind!(
        Layer::Login,
        ["k"],
        CommandId::MoveAuthenticationProviderPrev,
        "Previous authentication provider"
    );
    bind!(
        Layer::Login,
        ["up"],
        CommandId::MoveAuthenticationProviderPrev,
        "Previous authentication provider"
    );
    bind!(
        Layer::Login,
        ["j"],
        CommandId::MoveAuthenticationProviderNext,
        "Next authentication provider"
    );
    bind!(
        Layer::Login,
        ["down"],
        CommandId::MoveAuthenticationProviderNext,
        "Next authentication provider"
    );

    bind!(Layer::List, ["k"], CommandId::ListPrev, "Previous item");
    bind!(Layer::List, ["up"], CommandId::ListPrev, "Previous item");
    bind!(Layer::List, ["j"], CommandId::ListNext, "Next item");
    bind!(Layer::List, ["down"], CommandId::ListNext, "Next item");
    bind!(
        Layer::List,
        ["g", "g"],
        CommandId::ListFirst,
        "Go to first item"
    );
    bind!(
        Layer::List,
        ["g", "e"],
        CommandId::ListLast,
        "Go to last item"
    );

    bind!(Layer::Tabs, ["tab"], CommandId::MoveTabNext, "Next tab");
    bind!(
        Layer::Tabs,
        ["backtab"],
        CommandId::MoveTabPrev,
        "Previous tab"
    );

    bind!(
        Layer::Entries,
        ["r"],
        CommandId::RefreshTimeline,
        "Refresh timeline"
    );
    bind!(
        Layer::Entries,
        ["enter"],
        CommandId::OpenEntry,
        "Open entry with web browser"
    );
    bind!(
        Layer::Entries,
        ["space"],
        CommandId::BrowseEntry,
        "Browse entry with text browser"
    );

    bind!(
        Layer::Feeds,
        ["a"],
        CommandId::PromptFeedSubscription,
        "Add feed subscription"
    );
    bind!(
        Layer::Feeds,
        ["e"],
        CommandId::PromptFeedEdition,
        "Edit feed subscription"
    );
    bind!(
        Layer::Feeds,
        ["d"],
        CommandId::PromptFeedUnsubscription,
        "Delete feed subscription"
    );
    bind!(
        Layer::Feeds,
        ["r"],
        CommandId::ReloadSubscription,
        "Reload subscriptions"
    );
    bind!(
        Layer::Feeds,
        ["enter"],
        CommandId::OpenFeed,
        "Open selected feed"
    );

    bind!(
        Layer::Filter,
        ["h"],
        CommandId::MoveFilterRequirementPrev,
        "Previous requirement filter"
    );
    bind!(
        Layer::Filter,
        ["left"],
        CommandId::MoveFilterRequirementPrev,
        "Previous requirement filter"
    );
    bind!(
        Layer::Filter,
        ["l"],
        CommandId::MoveFilterRequirementNext,
        "Next requirement filter"
    );
    bind!(
        Layer::Filter,
        ["right"],
        CommandId::MoveFilterRequirementNext,
        "Next requirement filter"
    );
    bind!(
        Layer::Filter,
        ["c"],
        CommandId::ActivateCategoryFiltering,
        "Activate category filter"
    );
    bind!(
        Layer::Filter,
        ["/"],
        CommandId::ActivateSearchFiltering,
        "Activate search filter"
    );
    bind!(
        Layer::Filter,
        ["esc"],
        CommandId::DeactivateFiltering,
        "Deactivate filter"
    );

    bind!(
        Layer::UnsubscribePopup,
        ["h"],
        CommandId::MoveFeedUnsubscriptionPopupSelectionPrev,
        "Previous popup selection"
    );
    bind!(
        Layer::UnsubscribePopup,
        ["left"],
        CommandId::MoveFeedUnsubscriptionPopupSelectionPrev,
        "Previous popup selection"
    );
    bind!(
        Layer::UnsubscribePopup,
        ["l"],
        CommandId::MoveFeedUnsubscriptionPopupSelectionNext,
        "Next popup selection"
    );
    bind!(
        Layer::UnsubscribePopup,
        ["right"],
        CommandId::MoveFeedUnsubscriptionPopupSelectionNext,
        "Next popup selection"
    );
    bind!(
        Layer::UnsubscribePopup,
        ["enter"],
        CommandId::SelectFeedUnsubscriptionPopup,
        "Select popup item"
    );
    bind!(
        Layer::UnsubscribePopup,
        ["esc"],
        CommandId::CancelFeedUnsubscriptionPopup,
        "Cancel popup"
    );

    bind!(
        Layer::GhNotifications,
        ["enter"],
        CommandId::OpenGhNotification,
        "Open GitHub notification"
    );
    bind!(
        Layer::GhNotifications,
        ["A-enter"],
        CommandId::OpenGhNotificationAndMarkAsDone,
        "Open GitHub notification and mark as done"
    );
    bind!(
        Layer::GhNotifications,
        ["r"],
        CommandId::ReloadGhNotifications,
        "Reload GitHub notifications"
    );
    bind!(
        Layer::GhNotifications,
        ["d"],
        CommandId::MarkGhNotificationAsDone,
        "Mark GitHub notification as done"
    );
    bind!(
        Layer::GhNotifications,
        ["S-d"],
        CommandId::MarkAllGhNotificationsAsDone,
        "Mark all GitHub notifications as done"
    );
    bind!(
        Layer::GhNotifications,
        ["u"],
        CommandId::UnsubscribeGhThread,
        "Unsubscribe GitHub thread"
    );
    bind!(
        Layer::GhNotifications,
        ["f"],
        CommandId::OpenGhNotificationFilter,
        "Open GitHub notification filter popup"
    );

    bind!(
        Layer::GhNotificationFilterPopup,
        ["u", "n"],
        CommandId::ToggleGhNotificationFilterUnreadOnly,
        "Toggle unread filter"
    );
    bind!(
        Layer::GhNotificationFilterPopup,
        ["c", "l"],
        CommandId::ToggleGhNotificationFilterPullRequestClosed,
        "Toggle closed pull request filter"
    );
    bind!(
        Layer::GhNotificationFilterPopup,
        ["p", "a"],
        CommandId::ToggleGhNotificationFilterParticipatingOnly,
        "Toggle participating filter"
    );
    bind!(
        Layer::GhNotificationFilterPopup,
        ["p", "u"],
        CommandId::ToggleGhNotificationFilterVisibilityPublic,
        "Toggle public repository filter"
    );
    bind!(
        Layer::GhNotificationFilterPopup,
        ["p", "r"],
        CommandId::ToggleGhNotificationFilterVisibilityPrivate,
        "Toggle private repository filter"
    );
    bind!(
        Layer::GhNotificationFilterPopup,
        ["o", "p"],
        CommandId::ToggleGhNotificationFilterPullRequestOpen,
        "Toggle open pull request filter"
    );
    bind!(
        Layer::GhNotificationFilterPopup,
        ["m", "e"],
        CommandId::ToggleGhNotificationFilterReasonMentioned,
        "Toggle mentioned reason filter"
    );
    bind!(
        Layer::GhNotificationFilterPopup,
        ["m", "r"],
        CommandId::ToggleGhNotificationFilterPullRequestMerged,
        "Toggle merged pull request filter"
    );
    bind!(
        Layer::GhNotificationFilterPopup,
        ["r", "e"],
        CommandId::ToggleGhNotificationFilterReasonReviewRequested,
        "Toggle review requested reason filter"
    );
    bind!(
        Layer::GhNotificationFilterPopup,
        ["esc"],
        CommandId::CloseGhNotificationFilter,
        "Close GitHub notification filter popup"
    );
    bind!(
        Layer::GhNotificationFilterPopup,
        ["enter"],
        CommandId::CloseGhNotificationFilter,
        "Close GitHub notification filter popup"
    );

    config
}
