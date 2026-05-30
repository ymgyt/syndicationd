use std::fmt;

use serde::{Deserialize, Serialize};

use crate::CapabilitySet;

/// Opaque identifier assigned by the daemon to an opened session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Request body used by a client to open a daemon session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSessionRequest {
    required_capabilities: CapabilitySet,
}

impl OpenSessionRequest {
    pub fn new(required_capabilities: CapabilitySet) -> Self {
        Self {
            required_capabilities,
        }
    }

    pub fn required_capabilities(&self) -> &CapabilitySet {
        &self.required_capabilities
    }
}

/// Response body returned after the daemon accepts a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSessionResponse {
    session_id: SessionId,
    capabilities: CapabilitySet,
}

impl OpenSessionResponse {
    pub fn new(session_id: SessionId, capabilities: CapabilitySet) -> Self {
        Self {
            session_id,
            capabilities,
        }
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }
}

/// Request body used by a client to close a daemon session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseSessionRequest {
    session_id: SessionId,
}

impl CloseSessionRequest {
    pub fn new(session_id: SessionId) -> Self {
        Self { session_id }
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }
}

/// Response body returned after the daemon closes a session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseSessionResponse {}

impl CloseSessionResponse {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        CapabilitySet,
        session::{
            CloseSessionRequest, CloseSessionResponse, OpenSessionRequest, OpenSessionResponse,
            SessionId,
        },
    };

    #[test]
    fn serializes_session_contracts() {
        let cases = [
            (
                serde_json::to_value(OpenSessionRequest::new(CapabilitySet::new([
                    "timeline.read",
                ])))
                .unwrap(),
                json!({
                    "required_capabilities": {
                        "names": ["timeline.read"]
                    }
                }),
            ),
            (
                serde_json::to_value(OpenSessionResponse::new(
                    SessionId::new("session-1"),
                    CapabilitySet::new(["timeline.read"]),
                ))
                .unwrap(),
                json!({
                    "session_id": "session-1",
                    "capabilities": {
                        "names": ["timeline.read"]
                    }
                }),
            ),
            (
                serde_json::to_value(CloseSessionRequest::new(SessionId::new("session-1")))
                    .unwrap(),
                json!({
                    "session_id": "session-1"
                }),
            ),
            (
                serde_json::to_value(CloseSessionResponse::new()).unwrap(),
                json!({}),
            ),
        ];

        for (actual, expected) in cases {
            assert_eq!(actual, expected);
        }
    }
}
