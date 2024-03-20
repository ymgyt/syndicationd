use std::fmt::Display;
use synd_auth::device_flow::{DeviceAccessTokenResponse, DeviceAuthorizationResponse};

use crate::{
    application::{Direction, ListAction, RequestSequence},
    auth::AuthenticationProvider,
    client::{payload, query::subscription::SubscriptionOutput},
    types::Feed,
};

#[derive(Debug, Clone)]
pub enum Command {
    Quit,
    ResizeTerminal {
        columns: u16,
        rows: u16,
    },
    RenderThrobber,
    Idle,

    Authenticate,
    // Authenticate(AuthenticationProvider),
    MoveAuthenticationProvider(Direction),

    DeviceAuthorizationFlow {
        provider: AuthenticationProvider,
        device_authorization: DeviceAuthorizationResponse,
    },
    CompleteDevieAuthorizationFlow {
        provider: AuthenticationProvider,
        device_access_token: DeviceAccessTokenResponse,
    },

    MoveTabSelection(Direction),

    // Subscription
    MoveSubscribedFeed(Direction),
    MoveSubscribedFeedFirst,
    MoveSubscribedFeedLast,
    PromptFeedSubscription,
    PromptFeedUnsubscription,
    SubscribeFeed {
        url: String,
    },
    UnsubscribeFeed {
        url: String,
    },
    CompleteSubscribeFeed {
        feed: Feed,
        request_seq: RequestSequence,
    },
    CompleteUnsubscribeFeed {
        url: String,
        request_seq: RequestSequence,
    },
    FetchSubscription {
        after: Option<String>,
        first: i64,
    },
    UpdateSubscription {
        action: ListAction,
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
    UpdateEntries {
        action: ListAction,
        payload: payload::FetchEntriesPayload,
        request_seq: RequestSequence,
    },
    ReloadEntries,
    MoveEntry(Direction),
    MoveEntryFirst,
    MoveEntryLast,
    OpenEntry,

    HandleError {
        message: String,
        request_seq: Option<RequestSequence>,
    },
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::UpdateSubscription { .. } => f.write_str("UpdateSubscription"),
            Command::UpdateEntries { .. } => f.write_str("UpdateEntries"),
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
    pub fn prompt_feed_unsubscription() -> Self {
        Command::PromptFeedUnsubscription
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
}
