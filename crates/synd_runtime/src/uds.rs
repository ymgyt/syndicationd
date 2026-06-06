use std::path::{Path, PathBuf};

use crate::instance::RuntimeInstanceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UdsEndpoint {
    path: PathBuf,
}

impl UdsEndpoint {
    pub(crate) fn from_instance_id(root_dir: &Path, instance_id: &RuntimeInstanceId) -> Self {
        Self {
            path: root_dir.join(format!("api-{instance_id}.sock")),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use crate::{RuntimeDatabase, instance::RuntimeInstance};

    use super::*;

    mod socket_path {
        use super::*;

        #[test]
        fn from_instance_id() {
            let tmp = tempfile::tempdir().unwrap();
            let db = tmp.path().join("synd.db");
            let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(db)).unwrap();
            let root = tmp.path().join("runtime");

            let endpoint = UdsEndpoint::from_instance_id(&root, instance.id());

            assert_eq!(
                endpoint.path(),
                root.join(format!("api-{}.sock", instance.id()))
            );
        }
    }
}
