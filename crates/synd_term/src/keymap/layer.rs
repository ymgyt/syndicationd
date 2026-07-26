use std::{fmt, str::FromStr};

use serde::Deserialize;

/// Scope in which key bindings are active and prioritized.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Layer {
    App,
    Global,
    Login,
    Tabs,
    Entries,
    Feeds,
    Filter,
    CategoryFilter,
    SearchPrompt,
    UnsubscribePopup,
    GhNotifications,
    GhNotificationFilterPopup,
}

impl Layer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Global => "global",
            Self::Login => "login",
            Self::Tabs => "tabs",
            Self::Entries => "entries",
            Self::Feeds => "feeds",
            Self::Filter => "filter",
            Self::CategoryFilter => "category-filter",
            Self::SearchPrompt => "search-prompt",
            Self::UnsubscribePopup => "unsubscribe-popup",
            Self::GhNotifications => "github-notifications",
            Self::GhNotificationFilterPopup => "github-notification-filter-popup",
        }
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Layer {
    type Err = crate::keymap::KeymapError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "app" => Ok(Self::App),
            "global" => Ok(Self::Global),
            "login" => Ok(Self::Login),
            "tabs" => Ok(Self::Tabs),
            "entries" => Ok(Self::Entries),
            "feeds" => Ok(Self::Feeds),
            "filter" => Ok(Self::Filter),
            "category-filter" => Ok(Self::CategoryFilter),
            "search-prompt" => Ok(Self::SearchPrompt),
            "unsubscribe-popup" => Ok(Self::UnsubscribePopup),
            "github-notifications" => Ok(Self::GhNotifications),
            "github-notification-filter-popup" => Ok(Self::GhNotificationFilterPopup),
            unknown => Err(crate::keymap::KeymapError::UnknownLayer(unknown.to_owned())),
        }
    }
}

impl<'de> Deserialize<'de> for Layer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}
