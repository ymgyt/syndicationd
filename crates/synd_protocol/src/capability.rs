use std::fmt;

use serde::{Deserialize, Serialize};

/// Named timeline entries expose subscription labels in their feed metadata.
pub const NAMED_TIMELINES: &str = "timeline.named.v3";
/// Subscriptions accept a JSON object of labels.
pub const SUBSCRIPTION_WRITE: &str = "subscription.write.v2";
pub const FEED_REFRESH: &str = "feed.refresh";
pub const ENTRY_READ: &str = "entry.read";

pub fn local_api_capabilities() -> CapabilitySet {
    CapabilitySet::new([
        NAMED_TIMELINES,
        SUBSCRIPTION_WRITE,
        FEED_REFRESH,
        ENTRY_READ,
    ])
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

    pub fn insert(&mut self, name: &str) {
        if !self.names.iter().any(|existing| existing == name) {
            self.names.push(name.to_owned());
        }
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
    fn negotiates_named_timeline_revisions_and_single_entry_reads() {
        let legacy = CapabilitySet::new(["timeline.named", SUBSCRIPTION_WRITE, FEED_REFRESH]);
        let excerpts = CapabilitySet::new([NAMED_TIMELINES, SUBSCRIPTION_WRITE, FEED_REFRESH]);
        let current = local_api_capabilities();
        assert_eq!(excerpts.missing_from(&legacy).names(), [NAMED_TIMELINES]);
        assert_eq!(legacy.missing_from(&excerpts).names(), ["timeline.named"]);
        assert_eq!(current.missing_from(&excerpts).names(), [ENTRY_READ]);
        assert!(excerpts.missing_from(&current).is_empty());
    }
}
