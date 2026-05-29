use std::{io::ErrorKind, path::Path, process::ExitCode};

use anyhow::Context;
use clap::Args;
use synd_support::fs::FileSystem;
use synd_term::application::Cache;

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
    #[allow(clippy::unused_self)]
    pub fn run<FS>(self, config: &ConfigResolver, fs: &FS) -> ExitCode
    where
        FS: FileSystem + Clone,
    {
        let cache_dir = config.cache_dir();
        let log_file = config.log_file();
        ExitCode::from(self.clean(fs, &cache_dir, &log_file))
    }

    fn clean<FS>(self, fs: &FS, cache_dir: &Path, log: &Path) -> u8
    where
        FS: FileSystem + Clone,
    {
        if let Err(err) = self.try_clean(fs, cache_dir, log) {
            tracing::error!("{err}");
            1
        } else {
            0
        }
    }
    fn try_clean<FS>(self, fs: &FS, cache_dir: &Path, log: &Path) -> anyhow::Result<()>
    where
        FS: FileSystem + Clone,
    {
        let targets = self.targets();

        if targets.cache {
            let cache = Cache::with(cache_dir, fs.clone());
            cache
                .clean()
                .map_err(anyhow::Error::from)
                .with_context(|| format!("path: {}", cache_dir.display()))?;
        }

        if targets.logs {
            match fs.remove_file(log) {
                Ok(()) => {
                    tracing::info!("Remove {}", log.display());
                }
                Err(err) => match err.kind() {
                    ErrorKind::NotFound => {}
                    _ => {
                        return Err(anyhow::Error::from(err))
                            .with_context(|| format!("path: {}", log.display()));
                    }
                },
            }
        }

        Ok(())
    }

    fn targets(&self) -> CleanTargets {
        let default_all = !self.cache && !self.logs;
        CleanTargets {
            cache: default_all || self.cache,
            logs: default_all || self.logs,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CleanTargets {
    cache: bool,
    logs: bool,
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
        let exit_code = clean.clean(
            &fsimpl::FileSystem::new(),
            cache_dir.path(),
            log_file.path(),
        );
        assert_eq!(exit_code, 0);
        assert!(!log_file.path().exists());
    }

    #[test]
    fn ignore_log_file_not_found() {
        let clean = CleanCommand::logs();
        let cache_dir = TempDir::new().unwrap();
        let log_file = Path::new("./not_exists");
        let fs = MockFileSystem::default().with_remove_errors(log_file, io::ErrorKind::NotFound);
        let exit_code = clean.clean(&fs, cache_dir.path(), log_file);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn exit_code_on_permission_error() {
        let clean = CleanCommand::logs();
        let cache_dir = TempDir::new().unwrap();
        let log_file = Path::new("./not_allowed");
        let fs =
            MockFileSystem::default().with_remove_errors(log_file, io::ErrorKind::PermissionDenied);
        let exit_code = clean.clean(&fs, cache_dir.path(), log_file);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn cache_only_preserves_log_file() {
        let clean = CleanCommand::cache();
        let cache_dir = TempDir::new().unwrap();
        let log_file = NamedTempFile::new().unwrap();
        let exit_code = clean.clean(
            &fsimpl::FileSystem::new(),
            cache_dir.path(),
            log_file.path(),
        );
        assert_eq!(exit_code, 0);
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
