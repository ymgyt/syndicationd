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

/// Machine-readable reason for rejecting a session open request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenSessionErrorCode {
    MissingCapabilities,
}

/// Error body returned when the daemon rejects a session open request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSessionErrorResponse {
    code: OpenSessionErrorCode,
    missing_capabilities: CapabilitySet,
}

impl OpenSessionErrorResponse {
    pub fn from_missing_capabilities(missing_capabilities: CapabilitySet) -> Self {
        Self {
            code: OpenSessionErrorCode::MissingCapabilities,
            missing_capabilities,
        }
    }

    pub fn code(&self) -> OpenSessionErrorCode {
        self.code
    }

    pub fn missing_capabilities(&self) -> &CapabilitySet {
        &self.missing_capabilities
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

/// Machine-readable reason for rejecting a session close request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseSessionErrorCode {
    UnknownSession,
}

/// Error body returned when the daemon rejects a session close request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseSessionErrorResponse {
    code: CloseSessionErrorCode,
    session_id: SessionId,
}

impl CloseSessionErrorResponse {
    pub fn unknown_session(session_id: SessionId) -> Self {
        Self {
            code: CloseSessionErrorCode::UnknownSession,
            session_id,
        }
    }

    pub fn code(&self) -> CloseSessionErrorCode {
        self.code
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        CapabilitySet,
        session::{
            CloseSessionErrorResponse, CloseSessionRequest, CloseSessionResponse,
            OpenSessionErrorResponse, OpenSessionRequest, OpenSessionResponse, SessionId,
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
            (
                serde_json::to_value(OpenSessionErrorResponse::from_missing_capabilities(
                    CapabilitySet::new(["timeline.read"]),
                ))
                .unwrap(),
                json!({
                    "code": "missing_capabilities",
                    "missing_capabilities": {
                        "names": ["timeline.read"]
                    }
                }),
            ),
            (
                serde_json::to_value(CloseSessionErrorResponse::unknown_session(SessionId::new(
                    "session-1",
                )))
                .unwrap(),
                json!({
                    "code": "unknown_session",
                    "session_id": "session-1"
                }),
            ),
        ];

        for (actual, expected) in cases {
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn detects_missing_capabilities() {
        let required = CapabilitySet::new(["timeline.read", "subscription.write"]);
        let available = CapabilitySet::new(["timeline.read"]);

        assert_eq!(
            required.missing_from(&available),
            CapabilitySet::new(["subscription.write"])
        );
    }
}
