use std::{
    io,
    ops::ControlFlow,
    path::{Path, PathBuf},
};
use synd_stdx::fs::{self, fsimpl};
use thiserror::Error;
use tracing::debug;

const NAMESPACES_DIR: &str = "namespaces";
const SYSTEM_DIR: &str = "system";

#[derive(Error, Debug)]
pub enum ProvisionError {
    #[error("failed to create: `{path}` {source}")]
    Create { source: io::Error, path: String },
}

pub(super) struct Provisioner<FS = fsimpl::FileSystem> {
    root: PathBuf,
    fs: FS,
}

impl Provisioner<fsimpl::FileSystem> {
    pub(super) fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            fs: fsimpl::FileSystem::new(),
        }
    }

    #[cfg(test)]
    fn with_fs<FS>(self, fs: FS) -> Provisioner<FS> {
        Provisioner {
            root: self.root,
            fs,
        }
    }
}

impl<FS: fs::FileSystem> Provisioner<FS> {
    pub(super) fn provision(&self) -> Result<(), ProvisionError> {
        self.provision_file_tree()
    }

    fn provision_file_tree(&self) -> Result<(), ProvisionError> {
        debug!(path = %self.root.display(), "Create root directory");

        self.fs
            .create_dir_all(self.root.as_path())
            .map_err(|err| ProvisionError::Create {
                source: err,
                path: self.root.display().to_string(),
            })?;

        self.toplevel_directories().try_for_each(|dir| {
            match self.fs.create_dir_all(dir.as_path()) {
                Ok(()) => ControlFlow::Continue(()),
                Err(err) => ControlFlow::Break(ProvisionError::Create {
                    source: err,
                    path: dir.display().to_string(),
                }),
            }
        });

        Ok(())
    }
}

impl<FS> Provisioner<FS> {
    fn toplevel_directories(&self) -> impl Iterator<Item = PathBuf> + '_ {
        [NAMESPACES_DIR, SYSTEM_DIR]
            .into_iter()
            .map(Path::new)
            .map(|p| self.root.join(p))
    }
}

#[cfg(test)]
mod tests {
    use mockall::Sequence;
    use synd_stdx::fs::MockFileSystem;

    use super::*;

    #[test]
    fn file_tree() {
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
            let expect = root.join(NAMESPACES_DIR);
            mock.expect_create_dir_all()
                .withf(move |p| p.as_ref() == expect)
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Ok(()));
        }
        {
            let expect = root.join(SYSTEM_DIR);
            mock.expect_create_dir_all()
                .withf(move |p| p.as_ref() == expect)
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Ok(()));
        }

        let prov = Provisioner::new(root).with_fs(mock);

        assert!(prov.provision().is_ok());
    }
}
