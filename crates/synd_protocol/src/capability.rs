use std::fmt;

use serde::{Deserialize, Serialize};

pub const TIMELINE_READ: &str = "timeline.read";
pub const SUBSCRIPTION_WRITE: &str = "subscription.write";
pub const FEED_REFRESH: &str = "feed.refresh";

pub fn local_api_capabilities() -> CapabilitySet {
    CapabilitySet::new([TIMELINE_READ, SUBSCRIPTION_WRITE, FEED_REFRESH])
}

/// Capability names negotiated across the client/server protocol.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    names: Vec<String>,
}

impl CapabilitySet {
    pub fn new(names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            names: names.into_iter().map(Into::into).collect(),
        }
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    #[must_use]
    pub fn missing_from(&self, available: &Self) -> Self {
        Self::new(
            self.names
                .iter()
                .filter(|name| !available.names.contains(name))
                .cloned(),
        )
    }
}

impl fmt::Display for CapabilitySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.names.is_empty() {
            return f.write_str("<none>");
        }

        f.write_str(&self.names.join(", "))
    }
}
