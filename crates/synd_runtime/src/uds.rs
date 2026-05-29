use std::path::{Path, PathBuf};

use crate::identity::DaemonIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UdsEndpoint {
    path: PathBuf,
}

impl UdsEndpoint {
    pub(crate) fn from_root(root_dir: &Path, identity: &DaemonIdentity) -> Self {
        Self {
            path: root_dir.join(format!("api-{}.sock", identity.key())),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use crate::{RuntimeDatabase, identity::DaemonIdentity};

    use super::*;

    #[test]
    fn derives_socket_path_from_root_and_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("synd.db");
        let identity = DaemonIdentity::from_database(&RuntimeDatabase::sqlite(db)).unwrap();
        let root = tmp.path().join("runtime");

        let endpoint = UdsEndpoint::from_root(&root, &identity);

        assert_eq!(
            endpoint.path(),
            root.join(format!("api-{}.sock", identity.key()))
        );
    }
}
