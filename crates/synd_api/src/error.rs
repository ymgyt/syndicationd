use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    RegistryDb(#[from] synd_registry::RegistryDbError),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),

    #[error(transparent)]
    Github(#[from] crate::client::github::GithubClientError),

    #[error("tls config is invalid: {source}")]
    TlsConfig {
        #[source]
        source: std::io::Error,
    },

    #[error("local token must not be empty")]
    EmptyLocalToken,
}
