use crate::message::{MessageFrames, MessageType};

/// `Authenticate` is a message in which client requests the server
/// to perform authentication process.
// TODO: impl custom debug for mask credentials.
#[derive(Debug, Clone, PartialEq)]
pub struct Authenticate {
    username: String,
    password: String,
}

impl Authenticate {
    pub fn new<S1, S2>(username: S1, password: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

impl From<Authenticate> for MessageFrames {
    fn from(auth: Authenticate) -> Self {
        let mut frames = MessageFrames::new(MessageType::Authenticate, 2);

        frames.push_string(auth.username);
        frames.push_string(auth.password);

        frames
    }
}
