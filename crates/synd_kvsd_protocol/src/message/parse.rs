use std::vec;

use thiserror::Error;

use crate::message::{frame::Frame, MessageFrames, MessageType};

#[derive(Error, Debug)]
pub(super) enum ParseError {
    #[error("end of stream")]
    EndOfStream,
    #[error("unexpecte frame: {frame:?}")]
    UnexpectedFrame { frame: Frame },
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
