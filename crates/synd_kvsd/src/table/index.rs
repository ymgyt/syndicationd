#![expect(dead_code)]
use std::{borrow::Borrow, collections::HashMap, hash::Hash};

use thiserror::Error;
use tokio::io::AsyncReadExt;

use crate::table::entry::{Entry, EntryError};

#[derive(Error, Debug)]
pub(crate) enum IndexError {
    #[error("decode entry: {source}")]
    DecodeEntry { source: EntryError },
}

#[derive(Debug)]
pub(super) struct Index {
    // key to file offset mapping.
    entry_offsets: HashMap<String, usize>,
}

impl Index {
    pub(super) async fn from_reader<R: AsyncReadExt + Unpin>(
        mut reader: R,
    ) -> Result<Self, IndexError> {
        let mut entries = HashMap::new();
        let mut pos: usize = 0;
        loop {
            match Entry::decode_from(&mut reader).await {
                Ok((n, entry)) => {
                    // Ignore deleted entry
                    if entry.is_active() {
                        entries.insert(entry.take_key(), pos);
                    } else {
                        // Remove as there should be entry left before deleted
                        entries.remove(entry.take_key().as_str());
                    }
                    pos = pos.checked_add(n).unwrap();
                }
                Err(err) if err.is_eof() => {
                    return Ok(Index::new(entries));
                }
                Err(err) => {
                    return Err(IndexError::DecodeEntry { source: err });
                }
            }
        }
    }
    pub(super) fn add(&mut self, key: String, offset: usize) -> Option<usize> {
        self.entry_offsets.insert(key, offset)
    }

    pub(super) fn remove<Q>(&mut self, k: &Q) -> Option<usize>
    where
        String: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.entry_offsets.remove(k)
    }

    pub(super) fn lookup_offset(&self, key: &str) -> Option<usize> {
        self.entry_offsets.get(key).copied()
    }

    fn new(entry_offsets: HashMap<String, usize>) -> Self {
        Self { entry_offsets }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use synd_kvsd_protocol::KeyValue;

    #[tokio::test]
    async fn construct_index() {
        let mut entry1: Entry = KeyValue::try_from(("key1", "value1")).unwrap().into();
        let entry2: Entry = KeyValue::try_from(("key2", "value2")).unwrap().into();

        entry1.mark_deleted();

        let mut buf = Cursor::new(Vec::new());
        entry1.encode_to(&mut buf).await.unwrap();
        entry2.encode_to(&mut buf).await.unwrap();

        buf.set_position(0);

        let index = Index::from_reader(&mut buf).await.unwrap();

        let entry2_offset = index.lookup_offset("key2").unwrap();
        buf.set_position(entry2_offset as u64);

        let (_, decoded) = Entry::decode_from(&mut buf).await.unwrap();
        assert_eq!(entry2, decoded);

        assert_eq!(None, index.lookup_offset("key1"));
    }
}
