use std::{
    io,
    marker::PhantomData,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use synd_stdx::fs::{self, fsimpl};
use thiserror::Error;
use tracing::debug;

use crate::table::Namespace;
use state::{Provisioned, Unprovisioned};

mod state {
    pub(crate) struct Unprovisioned;
    pub(crate) struct Provisioned;
}

#[derive(Error, Debug)]
pub enum ProvisionError {
    #[error("failed to create: `{path}` {source}")]
    CreateFile { source: io::Error, path: String },
    #[error("failed to read: `{path}` {source}")]
    ReadFile { source: io::Error, path: String },
}

impl ProvisionError {
    fn read_dir(path: impl AsRef<Path>, err: io::Error) -> Self {
        ProvisionError::ReadFile {
            source: err,
            path: path.as_ref().display().to_string(),
        }
    }
}

struct FilePath {
    root: PathBuf,
    namespaces_dir: PathBuf,
    system_dir: PathBuf,
    default_table_dir: PathBuf,
}

impl FilePath {
    const NAMESPACES_DIR: &str = "namespaces";
    const SYSTEM_NAMESPACE: &str = "system";
    const DEFAULT_NAMESPACE: &str = "default";
    const DEFAULT_TABLE: &str = "default";

    fn new(root_dir: PathBuf) -> Self {
        let namespaces_dir = root_dir.join(Self::NAMESPACES_DIR);
        let system_dir = root_dir.join(Self::SYSTEM_NAMESPACE);
        let default_table_dir = namespaces_dir
            .join(Self::DEFAULT_NAMESPACE)
            .join(Self::DEFAULT_TABLE);

        Self {
            root: root_dir,
            namespaces_dir,
            system_dir,
            default_table_dir,
        }
    }
}

pub(super) struct Provisioner<State, FS = fsimpl::FileSystem> {
    paths: FilePath,
    fs: FS,
    state: PhantomData<State>,
}

impl Provisioner<Unprovisioned, fsimpl::FileSystem> {
    pub(super) fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            paths: FilePath::new(root.into()),
            fs: fsimpl::FileSystem::new(),
            state: PhantomData,
        }
    }

    #[cfg(test)]
    fn with_fs<FS>(self, fs: FS) -> Provisioner<Unprovisioned, FS> {
        Provisioner {
            paths: self.paths,
            fs,
            state: self.state,
        }
    }
}

impl<FS: fs::FileSystem> Provisioner<Unprovisioned, FS> {
    pub(super) fn provision(self) -> Result<Provisioner<Provisioned, FS>, ProvisionError> {
        self.provision_file_tree()?;
        Ok(Provisioner {
            paths: self.paths,
            fs: self.fs,
            state: PhantomData,
        })
    }

    fn provision_file_tree(&self) -> Result<(), ProvisionError> {
        debug!(path = %self.paths.root.display(), "Create root directory");

        self.fs
            .create_dir_all(self.paths.root.as_path())
            .map_err(|err| ProvisionError::CreateFile {
                source: err,
                path: self.paths.root.display().to_string(),
            })?;

        self.toplevel_directories()
            .try_for_each(|dir| match self.fs.create_dir_all(dir) {
                Ok(()) => ControlFlow::Continue(()),
                Err(err) => ControlFlow::Break(ProvisionError::CreateFile {
                    source: err,
                    path: dir.display().to_string(),
                }),
            });

        self.provision_default_file_tree()
    }

    fn provision_default_file_tree(&self) -> Result<(), ProvisionError> {
        let dir = self.default_table_dir();
        self.fs
            .create_dir_all(dir)
            .map_err(|err| ProvisionError::CreateFile {
                source: err,
                path: dir.display().to_string(),
            })
    }
}

impl<State, FS> Provisioner<State, FS> {
    fn toplevel_directories(&self) -> impl Iterator<Item = &Path> {
        [
            self.paths.namespaces_dir.as_path(),
            self.paths.system_dir.as_path(),
        ]
        .into_iter()
    }

    fn default_table_dir(&self) -> &Path {
        self.paths.default_table_dir.as_path()
    }

    fn namespace_dirs(&self) -> Result<impl Iterator<Item = (Namespace, PathBuf)>, ProvisionError> {
        let dir = self.paths.namespaces_dir.as_path();
        let dir_entries = std::fs::read_dir(dir)
            .map_err(|err| ProvisionError::read_dir(dir, err))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| ProvisionError::read_dir(dir, err))?;

        let mut namespaces = Vec::with_capacity(dir_entries.len());
        for entry in dir_entries {
            match entry.file_type() {
                Ok(file_type) if file_type.is_dir() => {
                    let ns = Namespace::from(entry.file_name().to_string_lossy());
                    namespaces.push((ns, entry.path()));
                }
                Ok(file_type) => {
                    debug!(
                        "Ignore {}({file_type:?}) as namespace directory",
                        entry.file_name().to_string_lossy(),
                    );
                    continue;
                }
                Err(err) => return Err(ProvisionError::read_dir(entry.path(), err)),
            }
        }

        Ok(namespaces.into_iter())
    }

    fn table_dirs_in_namespace(
        path: &Path,
    ) -> Result<impl Iterator<Item = PathBuf>, ProvisionError> {
        let dir_entries = std::fs::read_dir(path)
            .map_err(|err| ProvisionError::read_dir(path, err))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| ProvisionError::read_dir(path, err))?;

        let mut table_dirs = Vec::with_capacity(dir_entries.len());
        for entry in dir_entries {
            match entry.file_type() {
                Ok(file_type) if file_type.is_dir() => {
                    table_dirs.push(entry.path());
                }
                Ok(file_type) => {
                    debug!(
                        "Ignore {}({file_type:?}) as table directory",
                        entry.file_name().to_string_lossy(),
                    );
                    continue;
                }
                Err(err) => return Err(ProvisionError::read_dir(entry.path(), err)),
            }
        }

        Ok(table_dirs.into_iter())
    }
}

impl<FS> Provisioner<Provisioned, FS> {
    pub(super) fn table_dirs(
        &self,
    ) -> Result<impl Iterator<Item = (Namespace, PathBuf)>, ProvisionError> {
        let mut tables = Vec::new();

        for (namespace, path) in self.namespace_dirs()? {
            for table in Self::table_dirs_in_namespace(&path)? {
                tables.push((namespace.clone(), table));
            }
        }

        Ok(tables.into_iter())
    }
}

#[cfg(test)]
mod tests {
    use mockall::Sequence;
    use synd_stdx::fs::MockFileSystem;

    use super::*;

    #[test]
    fn provision_file_tree() {
        let root = tempfile::TempDir::new().unwrap().into_path().join("root");
        let mut seq = Sequence::new();
        let mut mock = MockFileSystem::new();
        {
            let expect = root.clone();
            mock.expect_create_dir_all()
                .withf(move |p| p.as_ref() == expect)
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Ok(()));
        }
        {
            let expect = root.join(FilePath::NAMESPACES_DIR);
            mock.expect_create_dir_all()
                .withf(move |p| p.as_ref() == expect)
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Ok(()));
        }
        {
            let expect = root.join(FilePath::SYSTEM_NAMESPACE);
            mock.expect_create_dir_all()
                .withf(move |p| p.as_ref() == expect)
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Ok(()));
        }
        {
            let expect = root
                .join(FilePath::NAMESPACES_DIR)
                .join(FilePath::DEFAULT_NAMESPACE)
                .join(FilePath::DEFAULT_TABLE);
            mock.expect_create_dir_all()
                .withf(move |p| p.as_ref() == expect)
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Ok(()));
        }

        let prov = Provisioner::new(root).with_fs(mock);

        assert!(prov.provision().is_ok());
    }

    #[test]
    fn iterate_table_dirs() {
        let root = tempfile::TempDir::new().unwrap().into_path().join("root");
        let prov = Provisioner::new(root).provision().unwrap();
        let tables = prov.table_dirs().unwrap().collect::<Vec<_>>();
        assert!(
            tables
                .iter()
                .any(|(ns, path)| ns == &Namespace::from(FilePath::DEFAULT_TABLE)
                    && path.ends_with(FilePath::DEFAULT_TABLE)),
            "should iterate default table"
        );
    }
}
