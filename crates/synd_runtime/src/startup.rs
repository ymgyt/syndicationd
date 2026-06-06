use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};

#[cfg(unix)]
use rustix::fs::{FlockOperation, flock};
use tracing::warn;

use crate::{Result, instance::RuntimeInstanceId};

/// Filesystem path used to serialize daemon startup for one runtime instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartupLockPath {
    path: PathBuf,
}

impl StartupLockPath {
    pub(crate) fn from_instance_id(root_dir: &Path, instance_id: &RuntimeInstanceId) -> Self {
        Self {
            path: root_dir.join(format!("api-{instance_id}.lock")),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

/// Attempts to acquire the startup lock without blocking the caller.
#[derive(Debug, Clone)]
pub(crate) struct StartupLockAcquirer {
    path: StartupLockPath,
}

impl StartupLockAcquirer {
    pub(crate) fn new(path: &StartupLockPath) -> Self {
        Self { path: path.clone() }
    }

    pub(crate) fn try_acquire(&self) -> Result<StartupLockAcquisition> {
        #[cfg(unix)]
        {
            self.create_parent_dir()?;

            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(self.path.path())?;

            match flock(&file, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => Ok(StartupLockAcquisition::Acquired(StartupLock::new(
                    self.path.clone(),
                    file,
                ))),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    Ok(StartupLockAcquisition::AlreadyHeld)
                }
                Err(error) => Err(std::io::Error::from(error).into()),
            }
        }

        #[cfg(not(unix))]
        {
            Ok(StartupLockAcquisition::UnsupportedTransport)
        }
    }

    fn create_parent_dir(&self) -> Result<()> {
        if let Some(parent) = self.path.path().parent() {
            std::fs::create_dir_all(parent)?;
        }

        Ok(())
    }
}

/// Result of trying to acquire a startup lock.
#[derive(Debug)]
pub(crate) enum StartupLockAcquisition {
    Acquired(StartupLock),
    AlreadyHeld,
    #[cfg(not(unix))]
    UnsupportedTransport,
}

/// Held startup lock. Dropping it releases the underlying file lock.
#[derive(Debug)]
pub(crate) struct StartupLock {
    path: StartupLockPath,
    file: File,
}

impl StartupLock {
    fn new(path: StartupLockPath, file: File) -> Self {
        Self { path, file }
    }

    pub(crate) fn path(&self) -> &StartupLockPath {
        &self.path
    }
}

impl Drop for StartupLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Err(error) = flock(&self.file, FlockOperation::Unlock) {
            warn!(
                startup_lock = %self.path.path().display(),
                error = %error,
                "Failed to unlock startup lock"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{RuntimeDatabase, instance::RuntimeInstance};

    use super::*;

    mod lock_path {
        use super::*;

        #[test]
        fn from_instance_id() {
            let tmp = tempfile::tempdir().unwrap();
            let db = tmp.path().join("synd.db");
            let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(db)).unwrap();
            let root = tmp.path().join("runtime");

            let lock_path = StartupLockPath::from_instance_id(&root, instance.id());

            assert_eq!(
                lock_path.path(),
                root.join(format!("api-{}.lock", instance.id()))
            );
        }
    }

    mod lock {
        use super::*;
        use core::assert_matches;

        #[test]
        fn holds_path() {
            let tmp = tempfile::tempdir().unwrap();
            let db = tmp.path().join("synd.db");
            let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(db)).unwrap();
            let lock_path = StartupLockPath::from_instance_id(tmp.path(), instance.id());

            let lock = match StartupLockAcquirer::new(&lock_path).try_acquire().unwrap() {
                StartupLockAcquisition::Acquired(lock) => lock,
                StartupLockAcquisition::AlreadyHeld => panic!("startup lock should be available"),
                #[cfg(not(unix))]
                StartupLockAcquisition::UnsupportedTransport => {
                    panic!("startup lock is unsupported")
                }
            };

            assert_eq!(lock.path(), &lock_path);
        }

        #[test]
        fn creates_parent_dir() {
            let tmp = tempfile::tempdir().unwrap();
            let lock_path = StartupLockPath {
                path: tmp.path().join("runtime").join("api-test.lock"),
            };

            let acquisition = StartupLockAcquirer::new(&lock_path).try_acquire().unwrap();

            assert_matches!(acquisition, StartupLockAcquisition::Acquired(_));
            assert!(lock_path.path().exists());
        }

        #[cfg(unix)]
        #[test]
        fn reports_contention() {
            let tmp = tempfile::tempdir().unwrap();
            let lock_path = StartupLockPath {
                path: tmp.path().join("api-test.lock"),
            };
            let first = StartupLockAcquirer::new(&lock_path).try_acquire().unwrap();
            let second = StartupLockAcquirer::new(&lock_path).try_acquire().unwrap();

            assert_matches!(first, StartupLockAcquisition::Acquired(_));
            assert_matches!(second, StartupLockAcquisition::AlreadyHeld);
        }

        #[cfg(unix)]
        #[test]
        fn allows_reacquisition() {
            let tmp = tempfile::tempdir().unwrap();
            let lock_path = StartupLockPath {
                path: tmp.path().join("api-test.lock"),
            };
            let first = StartupLockAcquirer::new(&lock_path).try_acquire().unwrap();

            drop(first);
            let second = StartupLockAcquirer::new(&lock_path).try_acquire().unwrap();

            assert_matches!(second, StartupLockAcquisition::Acquired(_));
        }
    }
}
