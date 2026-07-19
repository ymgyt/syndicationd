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

    bind!(Layer::Tabs, ["tab"], CommandId::MoveTabNext, "Next tab");
    bind!(
        Layer::Tabs,
        ["backtab"],
        CommandId::MoveTabPrev,
        "Previous tab"
    );

    bind!(
        Layer::Entries,
        ["k"],
        CommandId::MoveEntryPrev,
        "Previous entry"
    );
    bind!(
        Layer::Entries,
        ["up"],
        CommandId::MoveEntryPrev,
        "Previous entry"
    );
    bind!(
        Layer::Entries,
        ["j"],
        CommandId::MoveEntryNext,
        "Next entry"
    );
    bind!(
        Layer::Entries,
        ["down"],
        CommandId::MoveEntryNext,
        "Next entry"
    );
    bind!(
        Layer::Entries,
        ["r"],
        CommandId::ReloadEntries,
        "Reload entries"
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
        Layer::Entries,
        ["g", "g"],
        CommandId::MoveEntryFirst,
        "Go to first entry"
    );
    bind!(
        Layer::Entries,
        ["g", "e"],
        CommandId::MoveEntryLast,
        "Go to last entry"
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
        ["k"],
        CommandId::MoveSubscribedFeedPrev,
        "Previous feed"
    );
    bind!(
        Layer::Feeds,
        ["up"],
        CommandId::MoveSubscribedFeedPrev,
        "Previous feed"
    );
    bind!(
        Layer::Feeds,
        ["j"],
        CommandId::MoveSubscribedFeedNext,
        "Next feed"
    );
    bind!(
        Layer::Feeds,
        ["down"],
        CommandId::MoveSubscribedFeedNext,
        "Next feed"
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
        Layer::Feeds,
        ["g", "g"],
        CommandId::MoveSubscribedFeedFirst,
        "Go to first feed"
    );
    bind!(
        Layer::Feeds,
        ["g", "e"],
        CommandId::MoveSubscribedFeedLast,
        "Go to last feed"
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
        Layer::GithubNotifications,
        ["k"],
        CommandId::MoveGithubNotificationPrev,
        "Previous GitHub notification"
    );
    bind!(
        Layer::GithubNotifications,
        ["up"],
        CommandId::MoveGithubNotificationPrev,
        "Previous GitHub notification"
    );
    bind!(
        Layer::GithubNotifications,
        ["j"],
        CommandId::MoveGithubNotificationNext,
        "Next GitHub notification"
    );
    bind!(
        Layer::GithubNotifications,
        ["down"],
        CommandId::MoveGithubNotificationNext,
        "Next GitHub notification"
    );
    bind!(
        Layer::GithubNotifications,
        ["enter"],
        CommandId::OpenGithubNotification,
        "Open GitHub notification"
    );
    bind!(
        Layer::GithubNotifications,
        ["A-enter"],
        CommandId::OpenGithubNotificationWithDone,
        "Open GitHub notification and mark as done"
    );
    bind!(
        Layer::GithubNotifications,
        ["r"],
        CommandId::ReloadGithubNotifications,
        "Reload GitHub notifications"
    );
    bind!(
        Layer::GithubNotifications,
        ["d"],
        CommandId::MarkGithubNotificationAsDone,
        "Mark GitHub notification as done"
    );
    bind!(
        Layer::GithubNotifications,
        ["S-d"],
        CommandId::MarkGithubNotificationAsDoneAll,
        "Mark all GitHub notifications as done"
    );
    bind!(
        Layer::GithubNotifications,
        ["u"],
        CommandId::UnsubscribeGithubThread,
        "Unsubscribe GitHub thread"
    );
    bind!(
        Layer::GithubNotifications,
        ["g", "g"],
        CommandId::MoveGithubNotificationFirst,
        "Go to first GitHub notification"
    );
    bind!(
        Layer::GithubNotifications,
        ["g", "e"],
        CommandId::MoveGithubNotificationLast,
        "Go to last GitHub notification"
    );
    bind!(
        Layer::GithubNotifications,
        ["f"],
        CommandId::OpenGithubNotificationFilterPopup,
        "Open GitHub notification filter popup"
    );

    bind!(
        Layer::GithubNotificationFilterPopup,
        ["u", "n"],
        CommandId::ToggleGithubNotificationFilterPopupIncludeUnread,
        "Toggle unread filter"
    );
    bind!(
        Layer::GithubNotificationFilterPopup,
        ["c", "l"],
        CommandId::ToggleGithubNotificationFilterPopupPullRequestClosed,
        "Toggle closed pull request filter"
    );
    bind!(
        Layer::GithubNotificationFilterPopup,
        ["p", "a"],
        CommandId::ToggleGithubNotificationFilterPopupParticipating,
        "Toggle participating filter"
    );
    bind!(
        Layer::GithubNotificationFilterPopup,
        ["p", "u"],
        CommandId::ToggleGithubNotificationFilterPopupVisibilityPublic,
        "Toggle public repository filter"
    );
    bind!(
        Layer::GithubNotificationFilterPopup,
        ["p", "r"],
        CommandId::ToggleGithubNotificationFilterPopupVisibilityPrivate,
        "Toggle private repository filter"
    );
    bind!(
        Layer::GithubNotificationFilterPopup,
        ["o", "p"],
        CommandId::ToggleGithubNotificationFilterPopupPullRequestOpen,
        "Toggle open pull request filter"
    );
    bind!(
        Layer::GithubNotificationFilterPopup,
        ["m", "e"],
        CommandId::ToggleGithubNotificationFilterPopupReasonMentioned,
        "Toggle mentioned reason filter"
    );
    bind!(
        Layer::GithubNotificationFilterPopup,
        ["m", "r"],
        CommandId::ToggleGithubNotificationFilterPopupPullRequestMerged,
        "Toggle merged pull request filter"
    );
    bind!(
        Layer::GithubNotificationFilterPopup,
        ["r", "e"],
        CommandId::ToggleGithubNotificationFilterPopupReasonReviewRequested,
        "Toggle review requested reason filter"
    );
    bind!(
        Layer::GithubNotificationFilterPopup,
        ["esc"],
        CommandId::CloseGithubNotificationFilterPopup,
        "Close GitHub notification filter popup"
    );
    bind!(
        Layer::GithubNotificationFilterPopup,
        ["enter"],
        CommandId::CloseGithubNotificationFilterPopup,
        "Close GitHub notification filter popup"
    );

    config
}
