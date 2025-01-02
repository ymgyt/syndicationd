use std::io;

use tokio::io::AsyncWriteExt;

use crate::message::{spec, MessageType};

pub(in crate::message) mod prefix {
    pub(in crate::message) const MESSAGE_START: u8 = b'*';
    pub(in crate::message) const FRAME_LENGTH: u8 = b'@';
    pub(in crate::message) const MESSAGE_TYPE: u8 = b'#';
    pub(in crate::message) const STRING: u8 = b'+';
    pub(in crate::message) const BYTES: u8 = b'$';
    pub(in crate::message) const TIME: u8 = b'T';
    pub(in crate::message) const NULL: u8 = b'|';
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
    pub(crate) async fn write<W>(self, mut writer: W) -> Result<(), io::Error>
    where
        W: AsyncWriteExt + Unpin,
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
                writer.write_u64(val.len() as u64).await?;
                writer.write_all(val.as_ref()).await?;
                writer.write_all(spec::DELIMITER).await
            }
            Frame::Time(val) => {
                let val = val.to_rfc3339();
                writer.write_u8(prefix::TIME).await?;
                writer.write_u64(val.len() as u64).await?;
                writer.write_all(val.as_bytes()).await?;
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

    pub(super) fn push_string(&mut self, s: impl Into<String>) {
        self.0.push(Frame::String(s.into()));
    }

    pub(super) fn push_bytes(&mut self, bytes: impl Into<Vec<u8>>) {
        self.0.push(Frame::Bytes(bytes.into()));
    }

    pub(super) fn push_time(&mut self, time: Time) {
        self.0.push(Frame::Time(time));
    }

    pub(super) fn push_time_or_null(&mut self, time: Option<Time>) {
        match time {
            Some(t) => self.push_time(t),
            None => self.push_null(),
        }
    }

    pub(super) fn push_null(&mut self) {
        self.0.push(Frame::Null);
    }
}
