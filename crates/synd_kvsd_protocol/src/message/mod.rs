mod frame;
pub(crate) use frame::{FrameError, MessageFrames};
mod cursor;
pub(crate) use cursor::Cursor;
mod ioext;
pub(crate) use ioext::MessageWriteExt;
mod parse;
mod payload;
pub use payload::authenticate::Authenticate;
mod spec;

use std::io;

use thiserror::Error;

use crate::message::parse::Parse;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MessageError {
    #[error("unknown message type {message_type}")]
    UnknownMessageType { message_type: u8 },
    #[error("parse: {message}")]
    ParseFrame { message: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MessageType {
    Ping = 1,
    Authenticate = 2,
    Success = 3,
    Fail = 4,
    Set = 5,
    Get = 6,
    Delete = 7,
}

impl From<MessageType> for u8 {
    fn from(value: MessageType) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for MessageType {
    type Error = MessageError;
    fn try_from(n: u8) -> Result<Self, Self::Error> {
        match n {
            1 => Ok(MessageType::Ping),
            2 => Ok(MessageType::Authenticate),
            3 => Ok(MessageType::Success),
            4 => Ok(MessageType::Fail),
            5 => Ok(MessageType::Set),
            6 => Ok(MessageType::Get),
            7 => Ok(MessageType::Delete),
            _ => Err(MessageError::UnknownMessageType { message_type: n }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    // Ping(Ping),
    Authenticate(Authenticate),
    // Success(Success),
    // Fail(Fail),
    // Set(Set),
    // Get(Get),
    // Delete(Delete),
}

impl From<Message> for MessageFrames {
    fn from(message: Message) -> Self {
        match message {
            Message::Authenticate(m) => m.into(),
        }
    }
}

impl Message {
    pub(crate) fn parse(frames: MessageFrames) -> Result<Message, MessageError> {
        let mut parse = Parse::new(frames);
        // skip message_start and frame_length
        parse.skip(2);
        let message_type = parse.message_type().ok_or(MessageError::ParseFrame {
            message: "message type not found",
        })?;

        let message = match message_type {
            MessageType::Authenticate => Message::Authenticate(
                Authenticate::parse_frames(&mut parse)
                    .map_err(|_| MessageError::ParseFrame { message: "TODO" })?,
            ),
            // TODO: impl
            _ => unimplemented!(),
        };

        Ok(message)
    }

    pub(crate) async fn write<W>(self, mut writer: W) -> Result<(), io::Error>
    where
        W: MessageWriteExt,
    {
        let frames: MessageFrames = self.into();

        for frame in frames {
            frame.write(&mut writer).await?;
        }

        Ok(())
    }
}
