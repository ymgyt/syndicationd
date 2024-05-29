use std::fmt::Display;
use synd_auth::device_flow::{DeviceAccessTokenResponse, DeviceAuthorizationResponse};
use synd_feed::types::{Category, FeedUrl};

use crate::{
    application::{Direction, Populate, RequestSequence},
    auth::{AuthenticationProvider, Credential},
    client::{
        mutation::subscribe_feed::SubscribeFeedInput, payload,
        query::subscription::SubscriptionOutput,
    },
    types::Feed,
};

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

    DeviceAuthorizationFlow {
        provider: AuthenticationProvider,
        device_authorization: DeviceAuthorizationResponse,
    },
    CompleteDevieAuthorizationFlow {
        provider: AuthenticationProvider,
        device_access_token: DeviceAccessTokenResponse,
    },
    RefreshCredential {
        credential: Credential,
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
    CompleteSubscribeFeed {
        feed: Feed,
        request_seq: RequestSequence,
    },
    CompleteUnsubscribeFeed {
        url: FeedUrl,
        request_seq: RequestSequence,
    },
    FetchSubscription {
        after: Option<String>,
        first: i64,
    },
    PopulateFetchedSubscription {
        populate: Populate,
        subscription: SubscriptionOutput,
        request_seq: RequestSequence,
    },
    ReloadSubscription,
    OpenFeed,

    // Entries
    FetchEntries {
        after: Option<String>,
        first: i64,
    },
    PopulateFetchedEntries {
        populate: Populate,
        payload: payload::FetchEntriesPayload,
        request_seq: RequestSequence,
    },
    ReloadEntries,
    MoveEntry(Direction),
    MoveEntryFirst,
    MoveEntryLast,
    OpenEntry,

    // Filter
    MoveFilterRequirement(Direction),
    ActivateCategoryFilterling,
    ActivateSearchFiltering,
    PromptChanged,
    DeactivateFiltering,
    ToggleFilterCategory {
        category: Category<'static>,
    },
    ActivateAllFilterCategories,
    DeactivateAllFilterCategories,

    // Theme
    RotateTheme,

    // Latest release check
    InformLatestRelease(update_informer::Version),

    HandleError {
        message: String,
        request_seq: Option<RequestSequence>,
    },
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::PopulateFetchedSubscription { .. } => {
                f.write_str("PopulateFetchedSubscription")
            }
            Command::PopulateFetchedEntries { .. } => f.write_str("PopulateFetchedEntries"),
            Command::CompleteDevieAuthorizationFlow { .. } => {
                f.write_str("CompleteDeviceAuthorizationFlow")
            }
            cmd => write!(f, "{cmd:?}"),
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
}
