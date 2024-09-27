use bytes::Buf as _;
use thiserror::Error;

use crate::message::{MessageError, MessageType};

#[derive(Error, Debug)]
pub enum FrameError {
    /// Not enough data is available to decode a message frames from buffer.
    #[error("incomplete")]
    Incomplete,
    #[error("invalid message type: {0}")]
    InvalidMessageType(#[from] MessageError),
    #[error("invalid frame: {0}")]
    Invalid(String),
    // Other(common::Error),
}

// Should support time type ?
pub(crate) type Time = chrono::DateTime<chrono::Utc>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Frame {
    MessageType(MessageType),
    String(String),
    Bytes(Vec<u8>),
    Time(Time),
    Null,
}

impl Frame {
    fn check(src: &mut ByteCursor) -> Result<(), FrameError> {
        match cursor::get_u8(src)? {
            frameprefix::MESSAGE_TYPE => {
                cursor::get_u8(src)?;
                Ok(())
            }
            frameprefix::STRING => {
                cursor::get_line(src)?;
                Ok(())
            }
            frameprefix::BYTES => {
                #[allow(clippy::cast_possible_truncation)]
                let len = cursor::get_decimal(src)? as usize;
                // skip bytes length + delimiter
                cursor::skip(src, len + 2)
            }
            frameprefix::TIME => {
                cursor::get_line(src)?;
                Ok(())
            }
            frameprefix::NULL => Ok(()),
            _ => unreachable!(),
        }
    }

    fn parse(src: &mut ByteCursor) -> Result<Frame, FrameError> {
        match cursor::get_u8(src)? {
            frameprefix::MESSAGE_TYPE => {
                Err(FrameError::Invalid("unexpected message type frame".into()))
            }
            frameprefix::STRING => {
                let line = cursor::get_line(src)?.to_vec();
                let string =
                    String::from_utf8(line).map_err(|e| FrameError::Invalid(e.to_string()))?;
                Ok(Frame::String(string))
            }
            frameprefix::BYTES => {
                #[allow(clippy::cast_possible_truncation)]
                let len = cursor::get_decimal(src)? as usize;
                let n = len + 2;
                if src.remaining() < n {
                    return Err(FrameError::Incomplete);
                }
                let value = Vec::from(&src.chunk()[..len]);

                cursor::skip(src, n)?;

                Ok(Frame::Bytes(value))
            }
            frameprefix::TIME => {
                use chrono::{DateTime, Utc};
                let line = cursor::get_line(src)?.to_vec();
                let string =
                    String::from_utf8(line).map_err(|e| FrameError::Invalid(e.to_string()))?;
                Ok(Frame::Time(
                    DateTime::parse_from_rfc3339(&string)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap(),
                ))
            }
            frameprefix::NULL => Ok(Frame::Null),
            _ => unreachable!(),
        }
    }
}

mod frameprefix {
    pub(super) const MESSAGE_FRAMES: u8 = b'*';
    pub(super) const MESSAGE_TYPE: u8 = b'#';
    pub(super) const STRING: u8 = b'+';
    pub(super) const BYTES: u8 = b'$';
    pub(super) const TIME: u8 = b'T';
    pub(super) const NULL: u8 = b'|';
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct MessageFrames(Vec<Frame>);

type ByteCursor<'a> = std::io::Cursor<&'a [u8]>;

impl IntoIterator for MessageFrames {
    type Item = Frame;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl MessageFrames {
    pub(crate) fn with_capacity(mt: MessageType, n: usize) -> Self {
        let mut v = Vec::with_capacity(n + 1);
        v.push(Frame::MessageType(mt));
        Self(v)
    }

    pub(crate) fn check_parse(src: &mut ByteCursor) -> Result<(), FrameError> {
        let frames_len = MessageFrames::ensure_prefix_format(src)?;

        for _ in 0..frames_len {
            Frame::check(src)?;
        }

        Ok(())
    }

    pub(crate) fn parse(src: &mut ByteCursor) -> Result<MessageFrames, FrameError> {
        #[allow(clippy::cast_possible_truncation)]
        let frames_len = (MessageFrames::ensure_prefix_format(src)? - 1) as usize;

        if cursor::get_u8(src)? != frameprefix::MESSAGE_TYPE {
            return Err(FrameError::Invalid("message type expected".into()));
        }
        let message_type = cursor::get_u8(src)?;
        let message_type =
            MessageType::try_from(message_type).map_err(FrameError::InvalidMessageType)?;

        let mut frames = MessageFrames::with_capacity(message_type, frames_len);

        for _ in 0..frames_len {
            frames.0.push(Frame::parse(src)?);
        }

        Ok(frames)
    }

    fn ensure_prefix_format(src: &mut ByteCursor) -> Result<u64, FrameError> {
        if cursor::get_u8(src)? != frameprefix::MESSAGE_FRAMES {
            return Err(FrameError::Invalid("message frames prefix expected".into()));
        }

        cursor::get_decimal(src)
    }
}

/// cursor utilities.
// TODO: impl to ByteCursor
mod cursor {
    use bytes::Buf as _;

    use super::*;
    use crate::message::spec;

    pub(super) fn get_u8(src: &mut ByteCursor) -> Result<u8, FrameError> {
        if !src.has_remaining() {
            return Err(FrameError::Incomplete);
        }
        Ok(src.get_u8())
    }

    pub(super) fn skip(src: &mut ByteCursor, n: usize) -> Result<(), FrameError> {
        if src.remaining() < n {
            return Err(FrameError::Incomplete);
        }
        src.advance(n);
        Ok(())
    }

    pub(super) fn get_decimal(src: &mut ByteCursor) -> Result<u64, FrameError> {
        let line = get_line(src)?;

        atoi::atoi::<u64>(line)
            .ok_or_else(|| FrameError::Invalid("invalid protocol decimal format".into()))
    }

    pub(super) fn get_line<'a>(src: &'a mut ByteCursor) -> Result<&'a [u8], FrameError> {
        #[allow(clippy::cast_possible_truncation)]
        let start = src.position() as usize;
        let end = src.get_ref().len() - 1;

        for i in start..end {
            if src.get_ref()[i] == spec::DELIMITER[0] && src.get_ref()[i + 1] == spec::DELIMITER[1]
            {
                src.set_position((i + 2) as u64);

                return Ok(&src.get_ref()[start..i]);
            }
        }

        Err(FrameError::Incomplete)
    }
}
