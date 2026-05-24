use std::{path::PathBuf, time::Duration};

#[derive(Clone, Debug)]
pub struct LocalApiConfig {
    pub sqlite_db: PathBuf,
    pub timeout: Duration,
}
