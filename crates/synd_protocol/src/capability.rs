use serde::{Deserialize, Serialize};

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
