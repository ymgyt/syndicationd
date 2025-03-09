use std::string::FromUtf8Error;

use thiserror::Error;

use crate::message::{Authenticate, Fail, Message, MessageError, MessageType, Ping, Set, Success};

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("end of stream")]
    EndOfStream,
    #[error("invalid message type: {0}")]
    InvalidMessageType(#[from] MessageError),
    #[error("invalid utf8: {0}")]
    InvalidUtf8(#[from] FromUtf8Error),
    #[error("expect frame: {0}")]
    Expect(&'static str),
    #[error("incomplete")]
    Incomplete,
}

impl ParseError {
    #[expect(clippy::needless_pass_by_value)]
    pub(crate) fn expect(err: nom::Err<nom::error::Error<&[u8]>>, frame: &'static str) -> Self {
        if err.is_incomplete() {
            ParseError::Incomplete
        } else {
            ParseError::Expect(frame)
        }
    }
}

pub(crate) struct Parser;

impl Parser {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn parse<'a>(&self, input: &'a [u8]) -> Result<(&'a [u8], Message), ParseError> {
        let (input, _start) =
            parse::message_start(input).map_err(|err| ParseError::expect(err, "message_start"))?;

        let (input, _frame_length) =
            parse::frame_length(input).map_err(|err| ParseError::expect(err, "frame_length"))?;

        let (input, message_type) =
            parse::message_type(input).map_err(|err| ParseError::expect(err, "message_type"))?;
        let message_type = MessageType::try_from(message_type)?;

        match message_type {
            MessageType::Ping => {
                Ping::parse(input).map(|(input, ping)| (input, Message::Ping(ping)))
            }
            MessageType::Authenticate => {
                Authenticate::parse(input).map(|(input, auth)| (input, Message::Authenticate(auth)))
            }
            MessageType::Success => {
                Success::parse(input).map(|(input, success)| (input, Message::Success(success)))
            }
            MessageType::Fail => {
                Fail::parse(input).map(|(input, fail)| (input, Message::Fail(fail)))
            }
            MessageType::Set => Set::parse(input).map(|(input, set)| (input, Message::Set(set))),
            MessageType::Get => todo!(),
            MessageType::Delete => todo!(),
        }
    }
}

pub(super) mod parse {
    use nom::{
        IResult, Parser as _,
        bytes::streaming::{tag, take},
        combinator::{map, peek},
        number::streaming::{be_u64, u8},
        sequence::{pair, preceded, terminated},
    };

    use crate::message::{frame::prefix, spec};
    pub(super) fn message_start(input: &[u8]) -> IResult<&[u8], &[u8]> {
        tag([prefix::MESSAGE_START].as_slice())(input)
    }

    pub(super) fn frame_length(input: &[u8]) -> IResult<&[u8], u64> {
        preceded(tag([prefix::FRAME_LENGTH].as_slice()), u64).parse(input)
    }

    pub(super) fn message_type(input: &[u8]) -> IResult<&[u8], u8> {
        preceded(tag([prefix::MESSAGE_TYPE].as_slice()), u8).parse(input)
    }

    pub(crate) fn authenticate(input: &[u8]) -> IResult<&[u8], (&[u8], &[u8])> {
        pair(string, string).parse(input)
    }

    pub(crate) fn prefix(input: &[u8]) -> IResult<&[u8], u8> {
        u8(input)
    }

    pub(crate) fn peek_prefix(input: &[u8]) -> IResult<&[u8], u8> {
        peek(prefix).parse(input)
    }

    fn delimiter(input: &[u8]) -> IResult<&[u8], ()> {
        map(tag(spec::DELIMITER), |_| ()).parse(input)
    }

    fn u64(input: &[u8]) -> IResult<&[u8], u64> {
        be_u64(input)
    }

    pub(crate) fn string(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (input, len) = preceded(tag([prefix::STRING].as_slice()), u64).parse(input)?;
        terminated(take(len), delimiter).parse(input)
    }

    pub(crate) fn bytes(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
        let (input, len) = preceded(tag([prefix::BYTES].as_slice()), u64).parse(input)?;
        map(terminated(take(len), delimiter), <[u8]>::to_vec).parse(input)
    }

    pub(crate) fn time(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (input, len) = preceded(tag([prefix::TIME].as_slice()), u64).parse(input)?;
        terminated(take(len), delimiter).parse(input)
    }

    pub(crate) fn null(input: &[u8]) -> IResult<&[u8], ()> {
        map(tag([prefix::NULL].as_slice()), |_| ()).parse(input)
    }

    #[cfg(test)]
    mod tests {
        use chrono::DateTime;

        use crate::message::{MessageType, frame::Frame};

        use super::*;

        #[tokio::test]
        async fn parse_message_start() {
            let mut buf = Vec::new();
            let f = Frame::MessageStart;
            f.write(&mut buf).await.unwrap();

            let (remain, message) = message_start(buf.as_slice()).unwrap();
            assert!(remain.is_empty());
            assert_eq!(message, [prefix::MESSAGE_START].as_slice());

            let err = message_start(b"").unwrap_err();
            assert!(err.is_incomplete());
        }

        #[tokio::test]
        async fn parse_frame_length() {
            let mut buf = Vec::new();
            let f = Frame::Length(100);
            f.write(&mut buf).await.unwrap();

            let (remain, length) = frame_length(buf.as_slice()).unwrap();
            assert_eq!(length, 100);
            assert!(remain.is_empty());

            let err = frame_length(b"").unwrap_err();
            assert!(err.is_incomplete());
        }

        #[tokio::test]
        async fn parse_message_type() {
            let mut buf = Vec::new();
            let auth = MessageType::Authenticate;
            let f = Frame::MessageType(auth); // Replace `SomeType` with an actual variant of `MessageType`
            f.write(&mut buf).await.unwrap();

            let (remain, mt) = message_type(buf.as_slice()).unwrap();
            assert_eq!(mt, auth.into()); // Ensure `SomeType` matches the variant used above
            assert!(remain.is_empty());

            let err = message_type(b"").unwrap_err();
            assert!(err.is_incomplete());
        }

        #[tokio::test]
        async fn parse_string_frame() {
            for string_data in ["Hello", "", "\r\n"] {
                let mut buf = Vec::new();
                let f = Frame::String(string_data.to_owned());
                f.write(&mut buf).await.unwrap();

                let (remain, parsed_string) = string(buf.as_slice()).unwrap();
                assert_eq!(parsed_string, string_data.as_bytes());
                assert!(remain.is_empty());
            }
            let err = string(b"").unwrap_err();
            assert!(err.is_incomplete());
        }

        #[tokio::test]
        async fn parse_time_frame() {
            let mut buf = Vec::new();
            let t = DateTime::from_timestamp(1000, 0).unwrap();
            let f = Frame::Time(t);
            f.write(&mut buf).await.unwrap();

            let (remain, parsed_time) = time(buf.as_slice()).unwrap();
            assert_eq!(parsed_time, t.to_rfc3339().as_bytes());
            assert!(remain.is_empty());

            let err = time(b"").unwrap_err();
            assert!(err.is_incomplete());
        }
    }
}
