use std::{
    borrow::Borrow,
    io,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Serialize};
use synd_stdx::fs::{fsimpl, FileSystem};
use thiserror::Error;

use crate::{
    auth::{Credential, Unverified},
    config,
    ui::components::gh_notifications::GhNotificationFilterOptions,
};

#[derive(Debug, Error)]
pub enum PersistCacheError {
    #[error("io error: {path} {io} ")]
    Io { path: PathBuf, io: io::Error },
    #[error("serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum LoadCacheError {
    #[error("cache entry not found")]
    NotFound,
    #[error("io error: {path} {io}")]
    Io { path: PathBuf, io: io::Error },
    #[error("deserialize error: {0}")]
    Deserialize(#[from] serde_json::Error),
}

pub struct Cache<FS = fsimpl::FileSystem> {
    dir: PathBuf,
    fs: FS,
}

impl Cache<fsimpl::FileSystem> {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self::with(dir, fsimpl::FileSystem::new())
    }
}

impl<FS> Cache<FS>
where
    FS: FileSystem,
{
    pub fn with(dir: impl Into<PathBuf>, fs: FS) -> Self {
        Self {
            dir: dir.into(),
            fs,
        }
    }

    /// Persist credential in filesystem.
    /// This is blocking operation.
    pub fn persist_credential(
        &self,
        cred: impl Borrow<Credential>,
    ) -> Result<(), PersistCacheError> {
        self.persist(&self.credential_file(), cred.borrow())
    }

    pub(crate) fn persist_gh_notification_filter_options(
        &self,
        options: impl Borrow<GhNotificationFilterOptions>,
    ) -> Result<(), PersistCacheError> {
        self.persist(&self.gh_notification_filter_option_file(), options.borrow())
    }

    fn persist<T>(&self, path: &Path, entry: &T) -> Result<(), PersistCacheError>
    where
        T: ?Sized + Serialize,
    {
        if let Some(parent) = path.parent() {
            self.fs
                .create_dir_all(parent)
                .map_err(|err| PersistCacheError::Io {
                    path: parent.to_path_buf(),
                    io: err,
                })?;
        }

        self.fs
            .create_file(path)
            .map_err(|err| PersistCacheError::Io {
                path: path.to_path_buf(),
                io: err,
            })
            .and_then(|mut file| {
                serde_json::to_writer(&mut file, entry).map_err(PersistCacheError::Serialize)
            })
    }

    /// Load credential from filesystem.
    /// This is blocking operation.
    pub fn load_credential(&self) -> Result<Unverified<Credential>, LoadCacheError> {
        self.load::<Credential>(&self.credential_file())
            .map(Unverified::from)
    }

    pub(crate) fn load_gh_notification_filter_options(
        &self,
    ) -> Result<GhNotificationFilterOptions, LoadCacheError> {
        self.load(&self.gh_notification_filter_option_file())
    }

    fn load<T>(&self, path: &Path) -> Result<T, LoadCacheError>
    where
        T: DeserializeOwned,
    {
        self.fs
            .open_file(path)
            .map_err(|err| LoadCacheError::Io {
                io: err,
                path: path.to_path_buf(),
            })
            .and_then(|mut file| {
                serde_json::from_reader::<_, T>(&mut file).map_err(LoadCacheError::Deserialize)
            })
    }

    fn credential_file(&self) -> PathBuf {
        self.dir.join(config::cache::CREDENTIAL_FILE)
    }

    fn gh_notification_filter_option_file(&self) -> PathBuf {
        self.dir
            .join(config::cache::GH_NOTIFICATION_FILTER_OPTION_FILE)
    }

    /// Remove all cache files
    pub(crate) fn clean(&self) -> io::Result<()> {
        // User can specify any directory as the cache
        // so instead of deleting the entire directory with `remove_dir_all`, delete files individually.
        match self.fs.remove_file(self.credential_file()) {
            Ok(()) => Ok(()),
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => Ok(()),
                _ => Err(err),
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::auth::Credential;

    use super::*;

    #[test]
    fn persist_then_load_credential() {
        let tmp = temp_dir();
        let cache = Cache::new(tmp);
        let cred = Credential::Github {
            access_token: "rust is fun".into(),
        };
        assert!(cache.persist_credential(&cred).is_ok());

        let loaded = cache.load_credential().unwrap();
        assert_eq!(loaded, Unverified::from(cred),);
    }

    #[test]
    fn filesystem_error() {
        let cache = Cache::new("/dev/null");
        assert!(cache
            .persist_credential(Credential::Github {
                access_token: "dummy".into(),
            })
            .is_err());
    }

    fn temp_dir() -> PathBuf {
        tempfile::TempDir::new().unwrap().into_path()
    }
}
