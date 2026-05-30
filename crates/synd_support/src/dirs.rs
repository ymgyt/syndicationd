use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use directories::ProjectDirs;

const APP_NAME: &str = "syndicationd";
const CONFIG_FILE_NAME: &str = "config.toml";
const SQLITE_DB_FILE_NAME: &str = "synd.db";
const LOG_FILE_NAME: &str = "synd.log";

/// Platform-specific application directories for syndicationd.
#[derive(Debug)]
pub struct SyndicationdDirs {
    project_dirs: ProjectDirs,
}

impl SyndicationdDirs {
    pub fn current() -> &'static Self {
        static DIRS: OnceLock<SyndicationdDirs> = OnceLock::new();

        DIRS.get_or_init(|| Self::new().expect("failed to resolve syndicationd directories"))
    }

    pub fn new() -> Option<Self> {
        ProjectDirs::from("", "", APP_NAME).map(|project_dirs| Self { project_dirs })
    }

    pub fn cache_dir(&self) -> &Path {
        self.project_dirs.cache_dir()
    }

    pub fn config_file(&self) -> PathBuf {
        self.project_dirs.config_dir().join(CONFIG_FILE_NAME)
    }

    pub fn sqlite_db(&self) -> PathBuf {
        self.project_dirs.data_dir().join(SQLITE_DB_FILE_NAME)
    }

    pub fn log_file(&self) -> PathBuf {
        self.project_dirs.data_dir().join(LOG_FILE_NAME)
    }

    pub fn runtime_dir(&self) -> Option<&Path> {
        self.project_dirs.runtime_dir()
    }

    pub fn runtime_dir_or_temp(&self) -> PathBuf {
        self.runtime_dir()
            .map_or_else(|| std::env::temp_dir().join(APP_NAME), Path::to_path_buf)
    }
}

#[cfg(test)]
mod tests {
    use crate::dirs::SyndicationdDirs;

    #[test]
    fn resolves_current_platform_directories() {
        let dirs = SyndicationdDirs::new().unwrap();

        assert!(dirs.cache_dir().is_absolute());
        assert!(dirs.config_file().ends_with("config.toml"));
        assert!(dirs.sqlite_db().ends_with("synd.db"));
        assert!(dirs.log_file().ends_with("synd.log"));
        assert!(dirs.runtime_dir_or_temp().is_absolute());
    }
}
