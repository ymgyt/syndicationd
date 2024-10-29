use std::vec;

use thiserror::Error;

use crate::message::{frame::Frame, Message, MessageError, MessageFrames, MessageType};

#[derive(Error, Debug)]
pub(super) enum ParseError {
    #[error("end of stream")]
    EndOfStream,
    #[error("unexpecte frame: {frame:?}")]
    UnexpectedFrame { frame: Frame },
    #[error("invalid message type: {0}")]
    InvalidMessageType(#[from] MessageError),
    #[error("expect frame: {0}")]
    Expect(&'static str),
    #[error("incomplete")]
    Incomplete,
}

impl ParseError {
    #[expect(clippy::needless_pass_by_value)]
    fn expect(err: nom::Err<nom::error::Error<&[u8]>>, frame: &'static str) -> Self {
        if err.is_incomplete() {
            ParseError::Incomplete
        } else {
            ParseError::Expect(frame)
        }
    }
}

pub(super) struct Parse {
    frames: vec::IntoIter<Frame>,
}
impl Parse {
    pub(super) fn new(message_frames: MessageFrames) -> Self {
        Self {
            frames: message_frames.into_iter(),
        }
    }

    pub(super) fn skip(&mut self, n: usize) {
        for _ in 0..n {
            self.frames.next();
        }
    }

    pub(super) fn message_type(&mut self) -> Option<MessageType> {
        self.next().ok().and_then(|frame| match frame {
            Frame::MessageType(mt) => Some(mt),
            _ => None,
        })
    }

    pub(crate) fn next_string(&mut self) -> Result<String, ParseError> {
        match self.next()? {
            Frame::String(s) => Ok(s),
            frame => Err(ParseError::UnexpectedFrame { frame }),
        }
    }
    /*
    pub(crate) fn next_bytes(&mut self) -> Result<Vec<u8>, ParseError> {
        match self.next()? {
            Frame::Bytes(val) => Ok(val),
            frame => Err(format!("unexpected frame. want bytes got {:?}", frame).into()),
        }
    }

    pub(crate) fn next_bytes_or_null(&mut self) -> Result<Option<Vec<u8>>, ParseError> {
        match self.next()? {
            Frame::Bytes(val) => Ok(Some(val)),
            Frame::Null => Ok(None),
            frame => Err(format!("unexpected frame. want (bytes|null) got {:?}", frame).into()),
        }
    }

    pub(crate) fn next_time_or_null(&mut self) -> Result<Option<Time>, ParseError> {
        match self.next()? {
            Frame::Time(time) => Ok(Some(time)),
            Frame::Null => Ok(None),
            frame => Err(format!("unexpected frame. want (time|null) got {:?} ", frame).into()),
        }
    }

    // Make sure that caller has parse all the frames.
    pub(crate) fn expect_consumed(&mut self) -> Result<()> {
        match self.next() {
            Ok(frame) => Err(ErrorKind::NetworkFraming(format!(
                "unparsed frame still remains {:?}",
                frame
            ))
            .into()),
            Err(ParseError::EndOfStream) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
    */

    fn next(&mut self) -> Result<Frame, ParseError> {
        self.frames.next().ok_or(ParseError::EndOfStream)
    }
}

pub(super) struct Parser;

#[expect(dead_code)]
impl Parser {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn parse(&self, input: &[u8]) -> Result<Message, ParseError> {
        let (input, _start) =
            parse::message_start(input).map_err(|err| ParseError::expect(err, "message_start"))?;

        let (input, _frame_length) =
            parse::frame_length(input).map_err(|err| ParseError::expect(err, "frame_length"))?;

        let (input, message_type) =
            parse::message_type(input).map_err(|err| ParseError::expect(err, "message_type"))?;
        let message_type = MessageType::try_from(message_type)?;

        match message_type {
            MessageType::Ping => todo!(),
            MessageType::Authenticate => parse::authenticate(input).map(Message::Authenticate),
            MessageType::Success => todo!(),
            MessageType::Fail => todo!(),
            MessageType::Set => todo!(),
            MessageType::Get => todo!(),
            MessageType::Delete => todo!(),
        }
    }
}

mod parse {
    use nom::{
        bytes::streaming::{tag, take},
        combinator::map,
        number::streaming::{be_u64, u8},
        sequence::{preceded, terminated},
        IResult, Parser as _,
    };

    use crate::message::{frame::prefix, parse::ParseError, spec, Authenticate};
    pub(super) fn message_start(input: &[u8]) -> IResult<&[u8], &[u8]> {
        tag([prefix::MESSAGE_START].as_slice())(input)
    }

    pub(super) fn frame_length(input: &[u8]) -> IResult<&[u8], u64> {
        preceded(tag([prefix::FRAME_LENGTH].as_slice()), u64).parse(input)
    }

    pub(super) fn message_type(input: &[u8]) -> IResult<&[u8], u8> {
        preceded(tag([prefix::MESSAGE_TYPE].as_slice()), u8).parse(input)
    }

    pub(super) fn authenticate(_input: &[u8]) -> Result<Authenticate, ParseError> {
        todo!()
    }

    fn delimiter(input: &[u8]) -> IResult<&[u8], ()> {
        map(tag(spec::DELIMITER), |_| ()).parse(input)
    }

    fn u64(input: &[u8]) -> IResult<&[u8], u64> {
        be_u64(input)
    }

    #[allow(dead_code)]
    fn string(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (input, len) = preceded(tag([prefix::STRING].as_slice()), u64).parse(input)?;
        terminated(take(len), delimiter).parse(input)
    }

    #[cfg(test)]
    mod tests {
        use crate::message::{frame::Frame, MessageType};

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
    }
}
