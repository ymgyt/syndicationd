use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::Context;
use clap::Args;
use synd_stdx::fs::FileSystem;

use crate::{application::Cache, config};

/// Clean cache and logs
#[derive(Args, Debug)]
pub struct CleanCommand {
    /// Cache directory
    #[arg(
        long,
        default_value = config::cache::dir().to_path_buf().into_os_string(),
    )]
    cache_dir: PathBuf,
}

impl CleanCommand {
    #[allow(clippy::unused_self)]
    pub fn run<FS>(self, fs: &FS) -> ExitCode
    where
        FS: FileSystem + Clone,
    {
        ExitCode::from(self.clean(fs, config::log_path().as_path()))
    }

    fn clean<FS>(self, fs: &FS, log: &Path) -> u8
    where
        FS: FileSystem + Clone,
    {
        if let Err(err) = self.try_clean(fs, log) {
            tracing::error!("{err}");
            1
        } else {
            0
        }
    }
    fn try_clean<FS>(self, fs: &FS, log: &Path) -> anyhow::Result<()>
    where
        FS: FileSystem + Clone,
    {
        let CleanCommand { cache_dir } = self;

        let cache = Cache::with(&cache_dir, fs.clone());
        cache
            .clean()
            .map_err(anyhow::Error::from)
            .with_context(|| format!("path: {}", cache_dir.display()))?;

        // remove log
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use synd_stdx::fs::fsimpl;
    use tempfile::{NamedTempFile, TempDir};

    use crate::filesystem::mock::MockFileSystem;

    use super::*;

    #[test]
    fn remove_log_file() {
        let clean = CleanCommand {
            cache_dir: TempDir::new().unwrap().keep(),
        };
        let log_file = NamedTempFile::new().unwrap();
        let exit_code = clean.clean(&fsimpl::FileSystem::new(), log_file.path());
        assert_eq!(exit_code, 0);
        assert!(!log_file.path().exists());
    }

    #[test]
    fn ignore_log_file_not_found() {
        let clean = CleanCommand {
            cache_dir: TempDir::new().unwrap().keep(),
        };
        let log_file = Path::new("./not_exists");
        let fs = MockFileSystem::default().with_remove_errors(log_file, io::ErrorKind::NotFound);
        let exit_code = clean.clean(&fs, log_file);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn exit_code_on_permission_error() {
        let clean = CleanCommand {
            cache_dir: TempDir::new().unwrap().keep(),
        };
        let log_file = Path::new("./not_allowed");
        let fs =
            MockFileSystem::default().with_remove_errors(log_file, io::ErrorKind::PermissionDenied);
        let exit_code = clean.clean(&fs, log_file);
        assert_eq!(exit_code, 1);
    }
}
