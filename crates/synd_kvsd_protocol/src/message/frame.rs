use std::io;

use thiserror::Error;

use crate::message::{cursor::Cursor, ioext::MessageWriteExt, spec, MessageError, MessageType};

pub(in crate::message) mod prefix {
    pub(in crate::message) const MESSAGE_START: u8 = b'*';
    pub(in crate::message) const FRAME_LENGTH: u8 = b'@';
    pub(in crate::message) const MESSAGE_TYPE: u8 = b'#';
    pub(in crate::message) const STRING: u8 = b'+';
    pub(in crate::message) const BYTES: u8 = b'$';
    pub(in crate::message) const TIME: u8 = b'T';
    pub(in crate::message) const NULL: u8 = b'|';
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// Not enough data is available to decode a message frames from buffer.
    #[error("incomplete")]
    Incomplete,
    #[error("invalid message type: {0}")]
    InvalidMessageType(#[from] MessageError),
    #[error("invalid frame: {0}")]
    Invalid(String),
    #[error("invalid u64: {0}")]
    InvalidU64(String),
}

// Should support time type ?
pub(crate) type Time = chrono::DateTime<chrono::Utc>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Frame {
    MessageStart,
    Length(u64),
    MessageType(MessageType),
    String(String),
    Bytes(Vec<u8>),
    Time(Time),
    Null,
}

impl Frame {
    fn check(src: &mut Cursor) -> Result<(), FrameError> {
        match src.u8()? {
            prefix::MESSAGE_TYPE => {
                src.u8()?;
                Ok(())
            }
            prefix::STRING => {
                #[allow(clippy::cast_possible_truncation)]
                let len = src.u64()? as usize;
                src.skip(len + spec::DELIMITER.len())
            }
            prefix::BYTES => {
                #[allow(clippy::cast_possible_truncation)]
                let len = src.u64()? as usize;
                // skip bytes length + delimiter
                src.skip(len + spec::DELIMITER.len())
            }
            prefix::TIME => {
                src.line()?;
                Ok(())
            }
            prefix::NULL => Ok(()),
            unexpected => Err(FrameError::Invalid(format!(
                "unexpected prefix: {unexpected}"
            ))),
        }
    }

    fn read(src: &mut Cursor) -> Result<Frame, FrameError> {
        match src.u8()? {
            prefix::MESSAGE_START => Ok(Frame::MessageStart),
            prefix::FRAME_LENGTH => {
                let len = src.u64()?;
                Ok(Frame::Length(len))
            }
            prefix::MESSAGE_TYPE => MessageType::try_from(src.u8()?)
                .map_err(FrameError::InvalidMessageType)
                .map(Frame::MessageType),
            prefix::STRING => {
                #[allow(clippy::cast_possible_truncation)]
                let len = src.u64()? as usize;
                let n = len + spec::DELIMITER.len();
                if src.remaining() < n {
                    return Err(FrameError::Incomplete);
                }
                let string = std::str::from_utf8(&src.chunk()[..len])
                    .map_err(|e| FrameError::Invalid(e.to_string()))?
                    .to_owned();

                src.skip(n)?;

                Ok(Frame::String(string))
            }
            prefix::BYTES => {
                #[allow(clippy::cast_possible_truncation)]
                let len = src.u64()? as usize;
                let n = len + spec::DELIMITER.len();
                if src.remaining() < n {
                    return Err(FrameError::Incomplete);
                }
                let value = Vec::from(&src.chunk()[..len]);

                src.skip(n)?;

                Ok(Frame::Bytes(value))
            }
            prefix::TIME => {
                use chrono::{DateTime, Utc};
                let line = src.line()?.to_vec();
                let string =
                    String::from_utf8(line).map_err(|e| FrameError::Invalid(e.to_string()))?;
                Ok(Frame::Time(
                    DateTime::parse_from_rfc3339(&string)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap(),
                ))
            }
            prefix::NULL => Ok(Frame::Null),
            _ => unreachable!(),
        }
    }

    pub(crate) async fn write<W>(self, mut writer: W) -> Result<(), io::Error>
    where
        W: MessageWriteExt,
    {
        match self {
            Frame::MessageStart => writer.write_u8(prefix::MESSAGE_START).await,
            Frame::Length(len) => {
                writer.write_u8(prefix::FRAME_LENGTH).await?;
                writer.write_u64(len).await
            }
            Frame::MessageType(mt) => {
                writer.write_u8(prefix::MESSAGE_TYPE).await?;
                writer.write_u8(mt.into()).await
            }
            Frame::String(val) => {
                writer.write_u8(prefix::STRING).await?;
                writer.write_u64(val.len() as u64).await?;
                writer.write_all(val.as_bytes()).await?;
                writer.write_all(spec::DELIMITER).await
            }
            Frame::Bytes(val) => {
                writer.write_u8(prefix::BYTES).await?;
                writer.write_u64m(val.len() as u64).await?;
                writer.write_all(val.as_ref()).await?;
                writer.write_all(spec::DELIMITER).await
            }
            Frame::Time(val) => {
                writer.write_u8(prefix::TIME).await?;
                writer.write_all(val.to_rfc3339().as_bytes()).await?;
                writer.write_all(spec::DELIMITER).await
            }
            Frame::Null => writer.write_u8(prefix::NULL).await,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct MessageFrames(Vec<Frame>);

impl IntoIterator for MessageFrames {
    type Item = Frame;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl MessageFrames {
    pub(super) fn new(mt: MessageType, capasity: usize) -> Self {
        let mut v = Vec::with_capacity(capasity + 3);
        let message_len = capasity + 1;

        v.push(Frame::MessageStart);
        v.push(Frame::Length(message_len as u64));
        v.push(Frame::MessageType(mt));
        MessageFrames(v)
    }

    pub(crate) fn check_parse(src: &mut Cursor) -> Result<(), FrameError> {
        let frames_len = MessageFrames::frames_len(src)?;

        for _ in 0..frames_len {
            Frame::check(src)?;
        }

        Ok(())
    }

    pub(crate) fn parse(src: &mut Cursor) -> Result<MessageFrames, FrameError> {
        #[allow(clippy::cast_possible_truncation)]
        let frames_len = (MessageFrames::frames_len(src)? - 1) as usize;

        let message_type = match Frame::read(src) {
            Ok(Frame::MessageType(mt)) => mt,
            Ok(frame) => return Err(FrameError::Invalid(format!("invalid frame {frame:?}"))),
            Err(err) => return Err(err),
        };

        let mut frames = MessageFrames::new(message_type, frames_len);

        for _ in 0..frames_len {
            frames.0.push(Frame::read(src)?);
        }

        Ok(frames)
    }

    fn frames_len(src: &mut Cursor) -> Result<u64, FrameError> {
        if src.u8()? != prefix::MESSAGE_START {
            return Err(FrameError::Invalid("message frames prefix expected".into()));
        }
        if src.u8()? != prefix::FRAME_LENGTH {
            return Err(FrameError::Invalid("message frame length expected".into()));
        }
        src.u64()
    }

    pub(super) fn push_string(&mut self, s: impl Into<String>) {
        self.0.push(Frame::String(s.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn write_read_u64() {
        for val in [0, 1, 1024, u64::MAX] {
            let mut buf = Vec::new();
            buf.write_u64m(val).await.unwrap();
            let mut cursor = Cursor::new(&buf);
            assert_eq!(cursor.u64(), Ok(val));
        }
    }

    #[tokio::test]
    async fn frame_message_type() {
        let types = vec![
            MessageType::Ping,
            MessageType::Authenticate,
            MessageType::Success,
            MessageType::Fail,
            MessageType::Set,
            MessageType::Get,
            MessageType::Delete,
        ];

        for ty in types {
            let mut buf = Vec::new();
            let frame = Frame::MessageType(ty);
            frame.write(&mut buf).await.unwrap();
            let mut cursor = Cursor::new(&buf);
            assert_eq!(Frame::read(&mut cursor), Ok(Frame::MessageType(ty)));
        }
    }
}
