use std::path::{Path, PathBuf};

use crate::identity::DaemonIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartupLockPath {
    path: PathBuf,
}

impl StartupLockPath {
    pub(crate) fn from_root(root_dir: &Path, identity: &DaemonIdentity) -> Self {
        Self {
            path: root_dir.join(format!("api-{}.lock", identity.key())),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug)]
pub(crate) struct StartupLock {
    path: StartupLockPath,
}

impl StartupLock {
    pub(crate) fn new(path: StartupLockPath) -> Self {
        Self { path }
    }

    pub(crate) fn path(&self) -> &StartupLockPath {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use crate::{RuntimeDatabase, identity::DaemonIdentity};

    use super::*;

    #[test]
    fn derives_startup_lock_path_from_root_and_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("synd.db");
        let identity = DaemonIdentity::from_database(&RuntimeDatabase::sqlite(db)).unwrap();
        let root = tmp.path().join("runtime");

        let lock_path = StartupLockPath::from_root(&root, &identity);

        assert_eq!(
            lock_path.path(),
            root.join(format!("api-{}.lock", identity.key()))
        );
    }

    #[test]
    fn startup_lock_holds_lock_path() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("synd.db");
        let identity = DaemonIdentity::from_database(&RuntimeDatabase::sqlite(db)).unwrap();
        let lock_path = StartupLockPath::from_root(tmp.path(), &identity);

        let lock = StartupLock::new(lock_path.clone());

        assert_eq!(lock.path(), &lock_path);
    }
}
