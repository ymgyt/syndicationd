use std::io;

use bytes::Buf;

use crate::message::{spec, FrameError};

pub(crate) struct Cursor<'a> {
    cursor: io::Cursor<&'a [u8]>,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        Self {
            cursor: io::Cursor::new(buf),
        }
    }

    pub(crate) fn position(&self) -> u64 {
        self.cursor.position()
    }

    pub(crate) fn set_position(&mut self, pos: u64) {
        self.cursor.set_position(pos);
    }

    pub(super) fn skip(&mut self, n: usize) -> Result<(), FrameError> {
        if self.cursor.remaining() < n {
            Err(FrameError::Incomplete)
        } else {
            self.cursor.advance(n);
            Ok(())
        }
    }

    pub(super) fn remaining(&self) -> usize {
        self.cursor.remaining()
    }

    pub(super) fn chunk(&self) -> &[u8] {
        self.cursor.chunk()
    }

    pub(super) fn u8(&mut self) -> Result<u8, FrameError> {
        if self.cursor.has_remaining() {
            Ok(self.cursor.get_u8())
        } else {
            Err(FrameError::Incomplete)
        }
    }

    pub(super) fn u64(&mut self) -> Result<u64, FrameError> {
        let line = self.line()?;
        atoi::atoi::<u64>(line).ok_or_else(|| FrameError::Invalid("invalid u64 line".into()))
    }

    /// Return the buffer up to the line delimiter.  
    /// If the line delimiter is not found within the buffer, return [`FrameError::Incomplete`].  
    /// When the line delimiter is found, set the cursor position to the next position after the line delimiter
    /// so that subsequent reads do not need to be aware of the line delimiter.
    pub(super) fn line(&mut self) -> Result<&'a [u8], FrameError> {
        let slice = *self.cursor.get_ref();
        #[allow(clippy::cast_possible_truncation)]
        let start = self.cursor.position() as usize;
        let end = slice.len() - (spec::DELIMITER.len() - 1);

        for i in start..end {
            if &slice[i..i + spec::DELIMITER.len()] == spec::DELIMITER {
                self.cursor.set_position((i + spec::DELIMITER.len()) as u64);
                return Ok(&slice[start..i]);
            }
        }

        Err(FrameError::Incomplete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8() {
        let buf = [128, 129];
        let mut cursor = Cursor::new(&buf[..]);
        assert_eq!(cursor.u8(), Ok(128));
        assert_eq!(cursor.u8(), Ok(129));
        assert_eq!(cursor.u8(), Err(FrameError::Incomplete));
        assert_eq!(cursor.u8(), Err(FrameError::Incomplete));
    }

    #[test]
    fn line() {
        let buf = [b'x', b'x', b'\r', b'\n', b'y'];
        let mut cursor = Cursor::new(&buf[..]);
        assert_eq!(cursor.line(), Ok([b'x', b'x'].as_slice()));
        assert_eq!(cursor.line(), Err(FrameError::Incomplete));
        assert_eq!(cursor.line(), Err(FrameError::Incomplete));
    }

    #[test]
    fn skip() {
        let buf = [b'_', b'_', b'a'];
        let mut cursor = Cursor::new(&buf[..]);
        assert_eq!(cursor.skip(2), Ok(()));
        assert_eq!(cursor.u8(), Ok(b'a'));
        assert_eq!(cursor.skip(1), Err(FrameError::Incomplete));
    }
}
