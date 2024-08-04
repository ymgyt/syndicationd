use std::{borrow::Borrow, io, path::PathBuf};

use crate::{
    auth::{Credential, CredentialError, Unverified},
    config,
    filesystem::{fsimpl, FileSystem},
    ui::components::gh_notifications::GhNotificationFilterOptions,
};

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
    pub fn persist_credential(&self, cred: impl Borrow<Credential>) -> Result<(), CredentialError> {
        let cred = cred.borrow();
        let path = self.credential_file();

        self.fs.create_dir_all(self.dir.as_path()).map_err(|err| {
            CredentialError::PersistCredential {
                io_err: err,
                path: self.dir.clone(),
            }
        })?;

        let mut file = self
            .fs
            .create_file(&path)
            .map_err(|err| CredentialError::PersistCredential { io_err: err, path })?;

        serde_json::to_writer(&mut file, cred).map_err(CredentialError::Serialize)
    }

    /// Load credential from filesystem.
    /// This is blocking operation.
    pub fn load_credential(&self) -> Result<Unverified<Credential>, CredentialError> {
        let path = self.credential_file();

        let mut file = self
            .fs
            .open_file(&path)
            .map_err(|err| CredentialError::Open { io_err: err, path })?;

        serde_json::from_reader::<_, Credential>(&mut file)
            .map_err(CredentialError::Deserialize)
            .map(Unverified::from)
    }

    fn credential_file(&self) -> PathBuf {
        self.dir.join(config::cache::CREDENTIAL_FILE)
    }

    pub(crate) fn persist_gh_notification_filter_options(
        &self,
        options: impl Borrow<GhNotificationFilterOptions>,
    ) -> anyhow::Result<()> {
        let options = options.borrow();
        let path = self.gh_notification_filter_option_file();

        self.fs.create_dir_all(self.dir.as_path())?;

        let mut file = self.fs.create_file(path)?;

        serde_json::to_writer(&mut file, options).map_err(anyhow::Error::from)
    }

    pub(crate) fn load_gh_notification_filter_options(
        &self,
    ) -> anyhow::Result<GhNotificationFilterOptions> {
        let path = self.gh_notification_filter_option_file();

        let mut file = self.fs.open_file(path)?;

        serde_json::from_reader::<_, GhNotificationFilterOptions>(&mut file)
            .map_err(anyhow::Error::from)
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
