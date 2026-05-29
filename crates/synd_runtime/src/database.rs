use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeDatabase {
    Sqlite { path: PathBuf },
}

impl RuntimeDatabase {
    pub fn sqlite(path: impl Into<PathBuf>) -> Self {
        Self::Sqlite { path: path.into() }
    }

    pub fn sqlite_path(&self) -> &Path {
        match self {
            Self::Sqlite { path } => path,
        }
    }
}
