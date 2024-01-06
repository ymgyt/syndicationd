use std::path::PathBuf;

pub mod api {
    pub const ENDPOINT: &str = "http://localhost:5959/gql";
}

pub mod github {
    pub const CLIENT_ID: &str = "6652e5931c88e528a851";
}

pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .map(|path| path.join(APP_PATH))
        .expect("Faled to get cache dire")
}

const APP_PATH: &str = "syndicationd/syndterm";
