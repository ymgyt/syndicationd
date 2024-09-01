pub mod cli;
pub mod error;

pub type Result<T, E = crate::kvsd::error::KvsdError> = std::result::Result<T, E>;

// pub use protocol::{Key, Value};

/*
pub(crate) mod common {
    pub(crate) type Result<T, E = crate::error::internal::Error> = std::result::Result<T, E>;

    pub(crate) type Error = crate::error::internal::Error;
    pub(crate) type ErrorKind = crate::error::internal::ErrorKind;

    pub use crate::error::KvsdError;

    pub(crate) type Time = chrono::DateTime<chrono::Utc>;

    pub use tracing::{debug, error, info, trace, warn};
}
*/
