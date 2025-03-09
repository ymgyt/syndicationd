use std::{borrow::Cow, path::Path};

use synd_stdx::prelude::*;
use thiserror::Error;
use tokio::{
    fs,
    io::{self, AsyncRead, AsyncSeek, AsyncSeekExt, AsyncWrite, SeekFrom},
};
use tracing::debug;

use crate::{
    table::{
        Namespace,
        index::{Index, IndexError},
    },
    uow::{UnitOfWork, UowError, UowReceiver},
};

#[derive(Error, Debug)]
pub(crate) enum TableError {
    #[error("open file: {source}")]
    OpenFile { source: io::Error },
    #[error("seek: {source}")]
    Seek { source: io::Error },
    #[error("index: {0}")]
    Index(#[from] IndexError),
    #[error("invalid directory: `{path}` {message}")]
    InvalidDirectory { path: String, message: String },
}

impl TableError {
    fn seek(source: io::Error) -> Self {
        TableError::Seek { source }
    }
}

/// `TableRef` uniquely identifiers a [`Table`].
pub(crate) struct TableRef<'a> {
    pub(crate) namespace: Namespace,
    pub(crate) name: Cow<'a, str>,
}

#[expect(dead_code)]
pub(crate) struct Table<File = fs::File> {
    name: String,
    file: File,
    index: Index,
}

impl<FS> Table<FS> {
    pub(crate) fn name(&self) -> &str {
        self.name.as_str()
    }
}

impl Table<fs::File> {
    const DEFAULT_FILE: &str = "default.kvsd";
    // TODO: impl constructor from directory
    pub(crate) async fn try_from_dir(path: impl AsRef<Path>) -> Result<Self, TableError> {
        let name = path
            .as_ref()
            .file_name()
            .ok_or_else(|| TableError::InvalidDirectory {
                path: path.as_ref().display().to_string(),
                message: "directory basename not found".to_owned(),
            })
            .and_then(|name| {
                name.to_str().ok_or_else(|| TableError::InvalidDirectory {
                    path: path.as_ref().display().to_string(),
                    message: "invalid utf8 basename".to_owned(),
                })
            })?;
        let path = path.as_ref().join(Self::DEFAULT_FILE);
        Self::load(name, path).await
    }

    async fn load(name: impl Into<String>, path: impl AsRef<Path>) -> Result<Self, TableError> {
        let f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.as_ref())
            .await
            .map_err(|err| TableError::OpenFile { source: err })?;

        Table::new(name, f).await
    }
}

impl<File> Table<File>
where
    File: AsyncWrite + AsyncRead + AsyncSeek + Unpin,
{
    pub(crate) async fn new(name: impl Into<String>, mut file: File) -> Result<Self, TableError> {
        let pos = file
            .seek(SeekFrom::Current(0))
            .await
            .map_err(TableError::seek)?;
        debug!("initial pos {}", pos);

        // TODO: Buffering
        let index = Index::from_reader(&mut file).await?;
        // TODO: summary
        debug!("{:?}", index);

        Ok(Self {
            name: name.into(),
            file,
            index,
            // receiver,
        })
    }

    pub(crate) async fn run(mut self, mut receiver: UowReceiver) {
        while let Some(uow) = receiver.recv().await {
            if let Err(err) = self.handle_uow(uow).await {
                error!("handle uow {}", err);
            }
        }
    }

    #[expect(clippy::unused_async)]
    async fn handle_uow(&mut self, _uow: UnitOfWork) -> Result<(), UowError> {
        info!("Handle unit of work");
        Ok(())
        /*
        match uow {
            UnitOfWork::Set(set) => {
                info!("{}", set.request);

                let old_value = match self.lookup_entry(&set.request.key).await? {
                    Some(entry) => {
                        let (_, value) = entry.take_key_value();
                        Some(value)
                    }
                    None => None,
                };

                let current = self.file.seek(SeekFrom::Current(0)).await?;
                trace!("Seek {}", current);

                let entry = Entry::new(set.request.key.clone(), set.request.value)?;
                entry.encode_to(&mut self.file).await?;

                self.index
                    .add(set.request.key.into_string(), current as usize);

                self.send_value(set.response_sender, Ok(old_value.map(Value::new_unchecked)))
            }
            UnitOfWork::Get(get) => {
                info!("{}", get.request);

                let entry = match self.lookup_entry(&get.request.key).await? {
                    Some(entry) => entry,
                    None => return self.send_value(get.response_sender, Ok(None)),
                };

                let (key, value) = entry.take_key_value();

                debug_assert_eq!(*get.request.key, key);

                self.send_value(get.response_sender, Ok(Some(Value::new_unchecked(value))))
            }
            UnitOfWork::Delete(delete) => {
                info!("{}", delete.request);

                let mut entry = match self.lookup_entry(&delete.request.key).await? {
                    Some(entry) => entry,
                    None => return self.send_value(delete.response_sender, Ok(None)),
                };

                let value = entry.mark_deleted();
                entry.encode_to(&mut self.file).await?;

                self.index.remove(delete.request.key.as_str());

                self.send_value(
                    delete.response_sender,
                    Ok(Some(Value::new(value.unwrap())?)),
                )
            }
            _ => unreachable!(),
        }
        */
    }

    /*
    fn send_value(
        &self,
        sender: Option<oneshot::Sender<Result<Option<Value>>>>,
        value: Result<Option<Value>>,
    ) -> Result<()> {
        sender
            .expect("response already sent")
            .send(value)
            .map_err(|_| ErrorKind::Internal("send to resp channel".to_owned()).into())
    }

    async fn lookup_entry(&mut self, key: &Key) -> Result<Option<Entry>> {
        let maybe_offset = self.index.lookup_offset(key);

        let offset = match maybe_offset {
            Some(offset) => offset,
            None => return Ok(None),
        };

        let current = self.file.seek(SeekFrom::Current(0)).await?;

        self.file.seek(SeekFrom::Start(offset as u64)).await?;
        let (_, entry) = Entry::decode_from(&mut self.file).await?;
        self.file.seek(SeekFrom::Start(current)).await?;

        Ok(Some(entry))
    }
    */
}
