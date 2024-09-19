// TODO: remove
#![expect(dead_code)]

use std::io::ErrorKind;

use chrono::Utc;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use synd_kvsd_protocol::{Key, KeyValue, Value};

#[derive(Error, Debug)]
pub(crate) enum EntryError {
    #[error("encode error: {source}")]
    Encode { source: std::io::Error },
    #[error("decode error: {source}")]
    Decode { source: std::io::Error },
    #[error("eof: {source}")]
    Eof { source: std::io::Error },
    #[error("invalid utf8 key: {0}")]
    InvalidUtf8Key(#[from] std::string::FromUtf8Error),
}

impl EntryError {
    fn encode(source: std::io::Error) -> Self {
        EntryError::Encode { source }
    }

    fn decode(source: std::io::Error) -> Self {
        EntryError::Decode { source }
    }

    fn eof(source: std::io::Error) -> Self {
        EntryError::Eof { source }
    }

    pub(super) fn is_eof(&self) -> bool {
        matches!(self, EntryError::Eof { .. })
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Invalid = 0,
    Active = 1,
    Deleted = 2,
}

impl From<u8> for State {
    fn from(n: u8) -> Self {
        match n {
            1 => State::Active,
            2 => State::Deleted,
            _ => State::Invalid,
        }
    }
}

/// store mata value for entry.
#[derive(PartialEq, Debug)]
struct Header {
    // key length.
    key_bytes: usize,
    // value length.
    value_bytes: usize,
    // entry crated timestamp.
    // milliseconds since January 1,1970 UTC
    timestamp_ms: i64,
    // entry state. for support delete operation.
    state: State,
    // check data integrity.
    crc_checksum: Option<u32>,
}

impl Header {
    fn body_len(&self) -> usize {
        self.key_bytes + self.value_bytes
    }
}

/// actual data provided by user.
#[derive(PartialEq, Debug)]
struct Body {
    key: String,
    value: Option<Box<[u8]>>,
}

impl Body {
    fn len(&self) -> usize {
        self.key.len()
            + match &self.value {
                Some(value) => value.len(),
                None => 0,
            }
    }
}

/// Entry represent unit of data that is subject to an operation.
#[derive(PartialEq, Debug)]
pub(super) struct Entry {
    header: Header,
    body: Body,
}

impl From<KeyValue> for Entry {
    fn from(kv: KeyValue) -> Self {
        Entry::new(kv.key, kv.value)
    }
}

impl Entry {
    const HEADER_BYTES: usize = 8 // key_bytes
        + 8 // value_bytes
        + 8 // timestamp_ms
        + 1 // state
        + 4 // crc_checksum
    ;

    pub(super) fn new(key: Key, value: Value) -> Self {
        let header = Header {
            key_bytes: key.len(),
            value_bytes: value.len(),
            timestamp_ms: Utc::now().timestamp_millis(),
            state: State::Active,
            crc_checksum: None,
        };

        let body = Body {
            key: key.into_string(),
            value: Some(value.into_boxed_bytes()),
        };

        let mut entry = Self { header, body };
        entry.header.crc_checksum = Some(entry.calc_crc_checksum());

        entry
    }

    pub(super) fn mark_deleted(&mut self) -> Option<Box<[u8]>> {
        let value = self.body.value.take();

        self.header.value_bytes = 0;
        self.header.timestamp_ms = Utc::now().timestamp_millis();
        self.header.state = State::Deleted;
        self.header.crc_checksum = Some(self.calc_crc_checksum());

        value
    }

    /// Write binary expression to writer.
    /// return written bytes.
    /// flush is left to the caller.
    pub(crate) async fn encode_to<W: AsyncWriteExt + Unpin>(
        &self,
        mut writer: W,
    ) -> Result<usize, EntryError> {
        // Assuming that the validation is done at the timeout entry construction.
        debug_assert!(self.assert());

        let mut n: usize = Entry::HEADER_BYTES;
        // Header
        writer
            .write_u64(self.header.key_bytes as u64)
            .await
            .map_err(EntryError::encode)?;
        writer
            .write_u64(self.header.value_bytes as u64)
            .await
            .map_err(EntryError::encode)?;
        writer
            .write_i64(self.header.timestamp_ms)
            .await
            .map_err(EntryError::encode)?;
        writer
            .write_u8(self.header.state as u8)
            .await
            .map_err(EntryError::encode)?;
        writer
            .write_u32(self.header.crc_checksum.unwrap_or(0))
            .await
            .map_err(EntryError::encode)?;

        // Body
        writer
            .write_all(self.body.key.as_bytes())
            .await
            .map_err(EntryError::encode)?;
        if let Some(value) = &self.body.value {
            writer.write_all(value).await.map_err(EntryError::encode)?;
        }
        n += self.body.len();

        Ok(n)
    }

    /// Construct Entry from reader.
    pub(super) async fn decode_from<R: AsyncReadExt + Unpin>(
        mut reader: R,
    ) -> Result<(usize, Self), EntryError> {
        // Assuming reader is buffered.
        //
        // calling order is important.
        // We can't like this for eval_order_dependence(https://rust-lang.github.io/rust-clippy/master/index.html#eval_order_dependence)
        // Header {
        //   key_bytes: reader.read_u64().await        <-- second
        //   value_bytes: reader.read_u64().await      <-- first
        // }
        #[allow(clippy::cast_possible_truncation)]
        let header = {
            let key_bytes = match reader.read_u64().await {
                Ok(n) => n as usize,
                Err(err) => match err.kind() {
                    ErrorKind::UnexpectedEof => return Err(EntryError::eof(err)),
                    _ => return Err(EntryError::decode(err)),
                },
            };
            let value_bytes = reader.read_u64().await.map_err(EntryError::decode)? as usize;
            let timestamp_ms = reader.read_i64().await.map_err(EntryError::decode)?;
            let state = State::from(reader.read_u8().await.map_err(EntryError::decode)?);
            let crc_checksum = reader
                .read_u32()
                .await
                .map(|n| if n == 0 { None } else { Some(n) })
                .map_err(EntryError::decode)?;

            Header {
                key_bytes,
                value_bytes,
                timestamp_ms,
                state,
                crc_checksum,
            }
        };

        let mut buf = Vec::with_capacity(header.body_len());
        reader
            .take(header.body_len() as u64)
            .read_to_end(buf.as_mut())
            .await
            .map_err(EntryError::decode)?;

        let value = buf.split_off(header.key_bytes);

        let key = String::from_utf8(buf)?;

        let value = if value.is_empty() {
            None
        } else {
            Some(value.into_boxed_slice())
        };

        let entry = Self {
            header,
            body: Body { key, value },
        };

        Ok((entry.encoded_len(), entry))
    }

    pub(super) fn is_active(&self) -> bool {
        self.header.state == State::Active
    }

    pub(super) fn take_key(self) -> String {
        self.body.key
    }

    pub(super) fn take_key_value(self) -> (String, Box<[u8]>) {
        (self.body.key, self.body.value.unwrap())
    }

    fn calc_crc_checksum(&self) -> u32 {
        let mut h = crc32fast::Hasher::new();
        h.update(
            [
                self.header.key_bytes.to_be_bytes(),
                self.header.value_bytes.to_be_bytes(),
                self.header.timestamp_ms.to_be_bytes(),
            ]
            .concat()
            .as_ref(),
        );

        h.update((self.header.state as u8).to_be_bytes().as_ref());
        h.update(self.body.key.as_bytes());
        if let Some(value) = &self.body.value {
            h.update(value);
        }
        h.finalize()
    }

    // Assert entry data consistency.
    fn assert(&self) -> bool {
        self.header.key_bytes == self.body.key.len()
            && self.header.value_bytes == self.body.value.as_ref().map_or(0, |v| v.len())
            && self.header.crc_checksum.unwrap_or(0) == self.calc_crc_checksum()
    }

    // Return assuming encoded bytes length.
    fn encoded_len(&self) -> usize {
        Entry::HEADER_BYTES + self.body.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fmt::Debug, io::Cursor};

    fn entry<T>(kv: T) -> Entry
    where
        T: TryInto<KeyValue>,
        T::Error: Debug,
    {
        let kv = kv.try_into().unwrap();
        Entry::from(kv)
    }

    #[test]
    fn from_key_value() {
        let entry = entry(("key", b"hello"));

        assert_eq!(entry.header.key_bytes, 3);
        assert_eq!(entry.header.value_bytes, 5);
        assert!(entry.assert());
    }

    #[tokio::test]
    async fn encode_decode() {
        let entry = entry(("key", b"hello"));

        let mut buf = Cursor::new(Vec::new());
        let written = entry.encode_to(&mut buf).await.unwrap();
        assert_eq!(written, entry.encoded_len());

        buf.set_position(0);
        let (_, decoded) = Entry::decode_from(&mut buf).await.unwrap();

        assert_eq!(entry, decoded);
        assert!(decoded.assert());
    }

    #[tokio::test]
    async fn delete() {
        let mut entry1 = entry(("kv1", "value1"));
        entry1.mark_deleted();

        let mut buf = Cursor::new(Vec::new());
        entry1.encode_to(&mut buf).await.unwrap();

        buf.set_position(0);

        let (_, decoded) = Entry::decode_from(&mut buf).await.unwrap();

        assert_eq!(entry1, decoded);
        assert_eq!(decoded.header.state, State::Deleted);
    }

    #[tokio::test]
    async fn decode_should_return_eof_on_empty_source() {
        let mut buf = Cursor::new(Vec::new());
        let err = Entry::decode_from(&mut buf).await.unwrap_err();
        assert!(err.is_eof());
    }
}
