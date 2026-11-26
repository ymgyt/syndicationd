use std::{io::ErrorKind, path::Path};

use anyhow::Context;
use clap::Args;
use synd_support::fs::FileSystem;
use synd_term::application::Cache;
use tracing::info;

use crate::config::ConfigResolver;

/// Clean cache and logs
#[derive(Args, Debug)]
pub struct CleanCommand {
    /// Remove cache files
    #[arg(long, action = clap::ArgAction::SetTrue)]
    cache: bool,
    /// Remove log file
    #[arg(long, action = clap::ArgAction::SetTrue)]
    logs: bool,
}

impl CleanCommand {
    pub fn run(
        self,
        config: &ConfigResolver,
        fs: &(impl FileSystem + Clone),
    ) -> anyhow::Result<()> {
        self.remove_files(fs, &config.cache_dir(), &config.log_file())
    }

    fn remove_files(
        self,
        fs: &(impl FileSystem + Clone),
        cache_dir: &Path,
        log: &Path,
    ) -> anyhow::Result<()> {
        let clean_all = !self.cache && !self.logs;

        if clean_all || self.cache {
            let cache = Cache::with(cache_dir, fs.clone());
            cache
                .clean()
                .with_context(|| format!("failed to clean cache in {}", cache_dir.display()))?;
        }

        if clean_all || self.logs {
            match fs.remove_file(log) {
                Ok(()) => {
                    info!("Remove {}", log.display());
                }
                Err(err) if err.kind() == ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(err)
                        .with_context(|| format!("failed to remove log file {}", log.display()));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use synd_support::fs::fsimpl;
    use tempfile::{NamedTempFile, TempDir};

    use super::*;

    #[derive(Default, Clone)]
    struct MockFileSystem {
        remove_errors: std::collections::HashMap<std::path::PathBuf, io::ErrorKind>,
    }

    impl MockFileSystem {
        fn with_remove_errors(
            mut self,
            path: impl Into<std::path::PathBuf>,
            err: io::ErrorKind,
        ) -> Self {
            self.remove_errors.insert(path.into(), err);
            self
        }
    }

    impl FileSystem for MockFileSystem {
        fn create_dir_all<P: AsRef<Path>>(&self, _path: P) -> io::Result<()> {
            unimplemented!()
        }

        fn create_file<P: AsRef<Path>>(&self, _path: P) -> io::Result<std::fs::File> {
            unimplemented!()
        }

        fn open_file<P: AsRef<Path>>(&self, _path: P) -> io::Result<std::fs::File> {
            unimplemented!()
        }

        fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
            let path = path.as_ref();
            match self.remove_errors.get(path) {
                Some(err) => Err(io::Error::from(*err)),
                None => Ok(()),
            }
        }
    }

    #[test]
    fn remove_log_file() {
        let clean = CleanCommand::all();
        let cache_dir = TempDir::new().unwrap();
        let log_file = NamedTempFile::new().unwrap();
        clean
            .remove_files(
                &fsimpl::FileSystem::new(),
                cache_dir.path(),
                log_file.path(),
            )
            .unwrap();
        assert!(!log_file.path().exists());
    }

    #[test]
    fn ignore_log_file_not_found() {
        let clean = CleanCommand::logs();
        let cache_dir = TempDir::new().unwrap();
        let log_file = Path::new("./not_exists");
        let fs = MockFileSystem::default().with_remove_errors(log_file, io::ErrorKind::NotFound);
        clean.remove_files(&fs, cache_dir.path(), log_file).unwrap();
    }

    #[test]
    fn propagates_permission_errors() {
        let clean = CleanCommand::logs();
        let cache_dir = TempDir::new().unwrap();
        let log_file = Path::new("./not_allowed");
        let fs =
            MockFileSystem::default().with_remove_errors(log_file, io::ErrorKind::PermissionDenied);
        let error = clean
            .remove_files(&fs, cache_dir.path(), log_file)
            .unwrap_err();
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn cache_only_preserves_log_file() {
        let clean = CleanCommand::cache();
        let cache_dir = TempDir::new().unwrap();
        let log_file = NamedTempFile::new().unwrap();
        clean
            .remove_files(
                &fsimpl::FileSystem::new(),
                cache_dir.path(),
                log_file.path(),
            )
            .unwrap();
        assert!(log_file.path().exists());
    }

    impl CleanCommand {
        fn all() -> Self {
            Self {
                cache: false,
                logs: false,
            }
        }

        fn cache() -> Self {
            Self {
                cache: true,
                logs: false,
            }
        }

        fn logs() -> Self {
            Self {
                cache: false,
                logs: true,
            }
        }
    }
}
