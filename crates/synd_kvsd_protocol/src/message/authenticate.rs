use crate::message::{
    parse::{Parse, ParseError},
    MessageFrames,
};

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

    // TODO: impl in trait
    pub(super) fn parse_frames(parse: &mut Parse) -> Result<Self, ParseError> {
        let username = parse.next_string()?;
        let password = parse.next_string()?;

        Ok(Authenticate::new(username, password))
    }
}

impl From<Authenticate> for MessageFrames {
    fn from(_m: Authenticate) -> Self {
        todo!()
    }
}
