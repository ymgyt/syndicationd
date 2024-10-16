use std::io;

use thiserror::Error;
use tokio::io::AsyncWriteExt;

use crate::message::{cursor::Cursor, spec, MessageError, MessageType};

mod prefix {
    pub(super) const MESSAGE_FRAMES: u8 = b'*';
    pub(super) const MESSAGE_TYPE: u8 = b'#';
    pub(super) const STRING: u8 = b'+';
    pub(super) const BYTES: u8 = b'$';
    pub(super) const TIME: u8 = b'T';
    pub(super) const NULL: u8 = b'|';
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
    fn check(src: &mut Cursor) -> Result<(), FrameError> {
        match src.u8()? {
            prefix::MESSAGE_TYPE => {
                src.u8()?;
                Ok(())
            }
            prefix::STRING => {
                src.line()?;
                Ok(())
            }
            prefix::BYTES => {
                #[allow(clippy::cast_possible_truncation)]
                let len = src.u64()? as usize;
                // skip bytes length + delimiter
                src.skip(len + 2)
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
            prefix::MESSAGE_TYPE => {
                Err(FrameError::Invalid("unexpected message type frame".into()))
            }
            prefix::STRING => {
                let line = src.line()?.to_vec();
                let string =
                    String::from_utf8(line).map_err(|e| FrameError::Invalid(e.to_string()))?;
                Ok(Frame::String(string))
            }
            prefix::BYTES => {
                #[allow(clippy::cast_possible_truncation)]
                let len = src.u64()? as usize;
                let n = len + 2;
                if src.remaining() < n {
                    return Err(FrameError::Incomplete);
                }
                let value = Vec::from(&src.chunk()[..len]);

                // TODO: debug assert delimiter

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
        W: AsyncWriteExt + Unpin,
    {
        match self {
            Frame::MessageType(mt) => {
                writer.write_u8(prefix::MESSAGE_TYPE).await?;
                writer.write_u8(mt.into()).await
            }
            Frame::String(val) => {
                writer.write_u8(prefix::STRING).await?;
                writer.write_all(val.as_bytes()).await?;
                writer.write_all(spec::DELIMITER).await
            }
            Frame::Bytes(val) => {
                writer.write_u8(prefix::BYTES).await?;
                Frame::write_u64(val.len() as u64, &mut writer).await?;
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

    async fn write_u64<W>(val: u64, mut writer: W) -> io::Result<()>
    where
        W: AsyncWriteExt + Unpin,
    {
        use std::io::Write;

        // for write u64::MAX
        let mut buf = [0u8; 20];
        let mut buf = std::io::Cursor::new(&mut buf[..]);
        write!(&mut buf, "{val}")?;

        let pos: usize = buf.position().try_into().unwrap();
        writer.write_all(&buf.get_ref()[..pos]).await?;
        writer.write_all(spec::DELIMITER).await
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
    pub(crate) fn with_capacity(mt: MessageType, n: usize) -> Self {
        let mut v = Vec::with_capacity(n + 1);
        v.push(Frame::MessageType(mt));
        Self(v)
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

        if src.u8()? != prefix::MESSAGE_TYPE {
            return Err(FrameError::Invalid("message type expected".into()));
        }
        let message_type = src.u8()?;
        let message_type =
            MessageType::try_from(message_type).map_err(FrameError::InvalidMessageType)?;

        let mut frames = MessageFrames::with_capacity(message_type, frames_len);

        for _ in 0..frames_len {
            frames.0.push(Frame::read(src)?);
        }

        Ok(frames)
    }

    fn frames_len(src: &mut Cursor) -> Result<u64, FrameError> {
        if src.u8()? != prefix::MESSAGE_FRAMES {
            return Err(FrameError::Invalid("message frames prefix expected".into()));
        }
        src.u64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn write_read_u64() {
        for val in [0, 1, 1024, u64::MAX] {
            let mut buf = Vec::new();
            Frame::write_u64(val, &mut buf).await.unwrap();
            let mut cursor = Cursor::new(&buf);
            assert_eq!(cursor.u64(), Ok(val));
        }
    }
}
