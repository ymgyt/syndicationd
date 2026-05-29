use std::{
    fmt,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{RuntimeDatabase, error::Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DaemonIdentity {
    canonical_database_path: PathBuf,
    key: InstanceKey,
}

impl DaemonIdentity {
    pub(crate) fn from_database(database: &RuntimeDatabase) -> Result<Self> {
        let canonical_database_path = canonical_database_path(database.sqlite_path())?;
        let key = InstanceKey::from_path(&canonical_database_path);

        Ok(Self {
            canonical_database_path,
            key,
        })
    }

    pub(crate) fn canonical_database_path(&self) -> &Path {
        &self.canonical_database_path
    }

    pub(crate) fn key(&self) -> &InstanceKey {
        &self.key
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct InstanceKey(String);

impl InstanceKey {
    fn from_path(path: &Path) -> Self {
        let digest = Sha256::digest(path.as_os_str().as_encoded_bytes());
        Self(hex_prefix(&digest, 32))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InstanceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn canonical_database_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return path.canonicalize().map_err(Into::into);
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name();
    let mut canonical = parent.canonicalize()?;

    if let Some(file_name) = file_name {
        canonical.push(file_name);
    }

    Ok(canonical)
}

fn hex_prefix(bytes: &[u8], chars: usize) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut out = String::with_capacity(chars);
    for byte in bytes {
        if out.len() == chars {
            break;
        }
        out.push(HEX[(byte >> 4) as usize] as char);
        if out.len() == chars {
            break;
        }
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_identity_from_existing_database_path() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("synd.db");
        std::fs::write(&db, "").unwrap();

        let identity = DaemonIdentity::from_database(&RuntimeDatabase::sqlite(&db)).unwrap();

        assert_eq!(
            identity.canonical_database_path(),
            db.canonicalize().unwrap()
        );
        assert_eq!(identity.key().as_str().len(), 32);
    }

    #[test]
    fn derives_identity_from_database_path_that_does_not_exist_yet() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("synd.db");

        let identity = DaemonIdentity::from_database(&RuntimeDatabase::sqlite(&db)).unwrap();

        assert_eq!(
            identity.canonical_database_path(),
            tmp.path().canonicalize().unwrap().join("synd.db")
        );
        assert_eq!(identity.key().as_str().len(), 32);
    }
}
