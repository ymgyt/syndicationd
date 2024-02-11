use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use directories::ProjectDirs;

pub mod api {
    pub const ENDPOINT: &str = "https://localhost:5959/graphql";
}

pub mod github {
    pub const CLIENT_ID: &str = "6652e5931c88e528a851";
}

pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Number of entries to fetch
pub const INITIAL_ENTRY_TO_FETCH: i64 = 200;

pub fn cache_dir() -> &'static Path {
    project_dirs().cache_dir()
}

pub fn log_path() -> PathBuf {
    project_dirs().data_dir().join("syndterm.log")
}

fn project_dirs() -> &'static ProjectDirs {
    static PROJECT_DIRS: OnceLock<ProjectDirs> = OnceLock::new();

    PROJECT_DIRS.get_or_init(|| {
        ProjectDirs::from("io", "ymgyt", "syndterm").expect("Failed to get project dirs")
    })
}
