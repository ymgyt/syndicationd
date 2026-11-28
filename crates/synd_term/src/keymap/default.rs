use super::{CommandId, KeymapConfig, Layer, config::binding};

#[expect(clippy::too_many_lines)]
pub(super) fn default_keymap_config() -> KeymapConfig {
    let mut config = KeymapConfig::default();

    macro_rules! bind {
        ($layer:expr, $keys:expr, $command:expr) => {
            config.bind($layer, binding($keys, $command, None));
        };
        ($layer:expr, $keys:expr, $command:expr, $desc:literal) => {
            config.bind($layer, binding($keys, $command, Some($desc)));
        };
    }

    bind!(Layer::App, ["C-c"], CommandId::Quit);
    bind!(Layer::App, ["S-r"], CommandId::ForceRedraw, "Redraw");

    bind!(Layer::Global, ["q"], CommandId::Quit);
    bind!(
        Layer::Global,
        ["S-t"],
        CommandId::RotateTheme,
        "Rotate theme"
    );

    bind!(Layer::Login, ["enter"], CommandId::Authenticate);
    bind!(
        Layer::Login,
        ["k"],
        CommandId::MoveAuthenticationProviderPrev
    );
    bind!(
        Layer::Login,
        ["up"],
        CommandId::MoveAuthenticationProviderPrev
    );
    bind!(
        Layer::Login,
        ["j"],
        CommandId::MoveAuthenticationProviderNext
    );
    bind!(
        Layer::Login,
        ["down"],
        CommandId::MoveAuthenticationProviderNext
    );

    bind!(Layer::List, ["k"], CommandId::ListPrev);
    bind!(Layer::List, ["up"], CommandId::ListPrev);
    bind!(Layer::List, ["j"], CommandId::ListNext);
    bind!(Layer::List, ["down"], CommandId::ListNext);
    bind!(Layer::List, ["g", "g"], CommandId::ListFirst, "First");
    bind!(Layer::List, ["g", "e"], CommandId::ListLast, "Last");

    bind!(Layer::Tabs, ["tab"], CommandId::MoveTabNext);
    bind!(Layer::Tabs, ["backtab"], CommandId::MoveTabPrev);

    bind!(Layer::Entries, ["r"], CommandId::RefreshTimeline, "Refresh");
    bind!(Layer::Entries, ["enter"], CommandId::OpenEntry);
    bind!(Layer::Entries, ["space"], CommandId::ReadEntry);

    bind!(Layer::Reader, ["esc"], CommandId::CloseReader, "Back");
    bind!(Layer::Reader, ["q"], CommandId::CloseReader, "Back");
    bind!(Layer::Reader, ["j"], CommandId::ReaderLineDown, "Down");
    bind!(Layer::Reader, ["down"], CommandId::ReaderLineDown, "Down");
    bind!(Layer::Reader, ["k"], CommandId::ReaderLineUp, "Up");
    bind!(Layer::Reader, ["up"], CommandId::ReaderLineUp, "Up");
    bind!(
        Layer::Reader,
        ["z", "j"],
        CommandId::ReaderHalfPageDown,
        "Half page down"
    );
    bind!(
        Layer::Reader,
        ["z", "k"],
        CommandId::ReaderHalfPageUp,
        "Half page up"
    );
    bind!(
        Layer::Reader,
        ["space"],
        CommandId::ReaderPageDown,
        "Page down"
    );
    bind!(
        Layer::Reader,
        ["pagedown"],
        CommandId::ReaderPageDown,
        "Page down"
    );
    bind!(
        Layer::Reader,
        ["pageup"],
        CommandId::ReaderPageUp,
        "Page up"
    );

    bind!(Layer::Feeds, ["a"], CommandId::PromptFeedSubscription);
    bind!(Layer::Feeds, ["e"], CommandId::PromptFeedEdition);
    bind!(Layer::Feeds, ["d"], CommandId::PromptFeedUnsubscription);
    bind!(Layer::Feeds, ["r"], CommandId::ReloadSubscription, "Reload");
    bind!(Layer::Feeds, ["enter"], CommandId::OpenFeed);

    bind!(
        Layer::Subscribe,
        ["enter"],
        CommandId::ConfirmSubscribeField,
        "Confirm"
    );
    bind!(
        Layer::Subscribe,
        ["tab"],
        CommandId::NextSubscribeField,
        "Next"
    );
    bind!(
        Layer::Subscribe,
        ["S-tab"],
        CommandId::PreviousSubscribeField,
        "Previous"
    );
    bind!(
        Layer::Subscribe,
        ["esc"],
        CommandId::CancelSubscribe,
        "Cancel"
    );

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
    bind!(Layer::Filter, ["/"], CommandId::ActivateSearchFiltering);
    bind!(
        Layer::Filter,
        ["esc"],
        CommandId::DeactivateFiltering,
        "Close filter"
    );

    bind!(
        Layer::UnsubscribePopup,
        ["tab"],
        CommandId::MoveFeedUnsubscriptionPopupSelectionNext,
        "Next"
    );
    bind!(
        Layer::UnsubscribePopup,
        ["S-tab"],
        CommandId::MoveFeedUnsubscriptionPopupSelectionPrev,
        "Previous"
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
        CommandId::OpenGhNotification
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
        CommandId::MarkGhNotificationAsDone
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
        CommandId::OpenGhNotificationFilter
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
