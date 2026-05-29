use std::fmt;

use serde::{Deserialize, Serialize};

/// Opaque registry identity that owns subscriptions.
///
/// API and UI layers decide how authenticated principals map to this value.
/// The registry does not model local users or authentication providers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriberId(String);

impl SubscriberId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SubscriberId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
