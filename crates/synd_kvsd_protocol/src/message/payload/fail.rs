use std::fmt;

use crate::message::{MessageFrames, MessageType, ParseError, parse::parse};

const UNDEFINED: &str = "UNDEFINED";
const UNAUTHENTICATED: &str = "UNAUTHENTICATED";
const UNEXPECTED_MESSAGE: &str = "UNEXPECTED_MESSAGE";

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FailCode {
    Undefined,
    Unauthenticated,
    UnexpectedMessage,
}

impl fmt::Display for FailCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                FailCode::Undefined => UNDEFINED,
                FailCode::Unauthenticated => UNAUTHENTICATED,
                FailCode::UnexpectedMessage => UNEXPECTED_MESSAGE,
            }
        )
    }
}

impl From<String> for FailCode {
    fn from(s: String) -> Self {
        match s.as_str() {
            UNAUTHENTICATED => FailCode::Unauthenticated,
            UNEXPECTED_MESSAGE => FailCode::UnexpectedMessage,
            _ => FailCode::Undefined,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fail {
    code: FailCode,
    message: String,
}

impl Fail {
    pub fn new(code: FailCode) -> Self {
        Self {
            code,
            message: String::new(),
        }
    }

    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }
}

impl From<Fail> for MessageFrames {
    fn from(fail: Fail) -> Self {
        let mut frames = MessageFrames::new(MessageType::Fail, 2);

        frames.push_string(fail.code.to_string());
        frames.push_string(fail.message);

        frames
    }
}

impl Fail {
    pub(crate) fn parse(input: &[u8]) -> Result<(&[u8], Self), ParseError> {
        let (input, code) =
            parse::string(input).map_err(|err| ParseError::expect(err, "fail_code"))?;
        let fail_code = String::from_utf8(code.to_vec()).map(FailCode::from)?;

        let (input, message) =
            parse::string(input).map_err(|err| ParseError::expect(err, "message"))?;
        let message = String::from_utf8(message.to_vec())?;

        Ok((input, Fail::new(fail_code).with_message(message)))
    }
}
