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

    bind!(Layer::App, ["C-c"], CommandId::Quit, "Quit");
    bind!(Layer::App, ["S-r"], CommandId::ForceRedraw, "Redraw");

    bind!(Layer::Global, ["q"], CommandId::Quit, "Quit");
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
        "Prev provider"
    );
    bind!(
        Layer::Login,
        ["up"],
        CommandId::MoveAuthenticationProviderPrev,
        "Prev provider"
    );
    bind!(
        Layer::Login,
        ["j"],
        CommandId::MoveAuthenticationProviderNext,
        "Next provider"
    );
    bind!(
        Layer::Login,
        ["down"],
        CommandId::MoveAuthenticationProviderNext,
        "Next provider"
    );

    bind!(Layer::List, ["k"], CommandId::ListPrev, "Prev");
    bind!(Layer::List, ["up"], CommandId::ListPrev, "Prev");
    bind!(Layer::List, ["j"], CommandId::ListNext, "Next");
    bind!(Layer::List, ["down"], CommandId::ListNext, "Next");
    bind!(Layer::List, ["g", "g"], CommandId::ListFirst, "First");
    bind!(Layer::List, ["g", "e"], CommandId::ListLast, "Last");

    bind!(Layer::Tabs, ["tab"], CommandId::MoveTabNext, "Next tab");
    bind!(
        Layer::Tabs,
        ["backtab"],
        CommandId::MoveTabPrev,
        "Previous tab"
    );

    bind!(Layer::Entries, ["r"], CommandId::RefreshTimeline, "Refresh");
    bind!(Layer::Entries, ["enter"], CommandId::OpenEntry, "Open");
    bind!(Layer::Entries, ["space"], CommandId::BrowseEntry, "Browse");

    bind!(
        Layer::Feeds,
        ["a"],
        CommandId::PromptFeedSubscription,
        "Subscribe"
    );
    bind!(Layer::Feeds, ["e"], CommandId::PromptFeedEdition, "Edit");
    bind!(
        Layer::Feeds,
        ["d"],
        CommandId::PromptFeedUnsubscription,
        "Unsubscribe"
    );
    bind!(Layer::Feeds, ["r"], CommandId::ReloadSubscription, "Reload");
    bind!(Layer::Feeds, ["enter"], CommandId::OpenFeed, "Open");

    bind!(
        Layer::Filter,
        ["h"],
        CommandId::MoveFilterRequirementPrev,
        "Requirement −"
    );
    bind!(
        Layer::Filter,
        ["left"],
        CommandId::MoveFilterRequirementPrev,
        "Requirement −"
    );
    bind!(
        Layer::Filter,
        ["l"],
        CommandId::MoveFilterRequirementNext,
        "Requirement +"
    );
    bind!(
        Layer::Filter,
        ["right"],
        CommandId::MoveFilterRequirementNext,
        "Requirement +"
    );
    bind!(
        Layer::Filter,
        ["c"],
        CommandId::ActivateCategoryFiltering,
        "Categories"
    );
    bind!(
        Layer::Filter,
        ["/"],
        CommandId::ActivateSearchFiltering,
        "Search"
    );
    bind!(
        Layer::Filter,
        ["esc"],
        CommandId::DeactivateFiltering,
        "Close filter"
    );

    bind!(
        Layer::UnsubscribePopup,
        ["h"],
        CommandId::MoveFeedUnsubscriptionPopupSelectionPrev,
        "Choice"
    );
    bind!(
        Layer::UnsubscribePopup,
        ["left"],
        CommandId::MoveFeedUnsubscriptionPopupSelectionPrev,
        "Choice"
    );
    bind!(
        Layer::UnsubscribePopup,
        ["l"],
        CommandId::MoveFeedUnsubscriptionPopupSelectionNext,
        "Choice"
    );
    bind!(
        Layer::UnsubscribePopup,
        ["right"],
        CommandId::MoveFeedUnsubscriptionPopupSelectionNext,
        "Choice"
    );
    bind!(
        Layer::UnsubscribePopup,
        ["enter"],
        CommandId::SelectFeedUnsubscriptionPopup,
        "Confirm"
    );
    bind!(
        Layer::UnsubscribePopup,
        ["esc"],
        CommandId::CancelFeedUnsubscriptionPopup,
        "Cancel"
    );

    bind!(
        Layer::GhNotifications,
        ["enter"],
        CommandId::OpenGhNotification,
        "Open"
    );
    bind!(
        Layer::GhNotifications,
        ["A-enter"],
        CommandId::OpenGhNotificationAndMarkAsDone,
        "Open & finish"
    );
    bind!(
        Layer::GhNotifications,
        ["r"],
        CommandId::ReloadGhNotifications,
        "Reload"
    );
    bind!(
        Layer::GhNotifications,
        ["d"],
        CommandId::MarkGhNotificationAsDone,
        "Done"
    );
    bind!(
        Layer::GhNotifications,
        ["S-d"],
        CommandId::MarkAllGhNotificationsAsDone,
        "Finish all"
    );
    bind!(
        Layer::GhNotifications,
        ["u"],
        CommandId::UnsubscribeGhThread,
        "Unsubscribe"
    );
    bind!(
        Layer::GhNotifications,
        ["f"],
        CommandId::OpenGhNotificationFilter,
        "Filters"
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
        "Apply filters"
    );
    bind!(
        Layer::GhNotificationFilterPopup,
        ["enter"],
        CommandId::CloseGhNotificationFilter,
        "Apply filters"
    );

    config
}
