use std::fmt;

use serde::{Deserialize, Serialize};

/// Named timelines with entry excerpts. The first revision exposed `Entry.summary`.
pub const NAMED_TIMELINES: &str = "timeline.named.v2";
pub const SUBSCRIPTION_WRITE: &str = "subscription.write";
pub const FEED_REFRESH: &str = "feed.refresh";

pub fn local_api_capabilities() -> CapabilitySet {
    CapabilitySet::new([NAMED_TIMELINES, SUBSCRIPTION_WRITE, FEED_REFRESH])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_and_excerpt_clients_require_different_daemons() {
        let legacy = CapabilitySet::new(["timeline.named", SUBSCRIPTION_WRITE, FEED_REFRESH]);
        let current = local_api_capabilities();
        assert_eq!(current.missing_from(&legacy).names(), [NAMED_TIMELINES]);
        assert_eq!(legacy.missing_from(&current).names(), ["timeline.named"]);
    }
}
