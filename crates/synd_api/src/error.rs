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

    #[error("{field} is required unless local mode is enabled")]
    TlsOptionRequired { field: &'static str },

    #[error("tls options are invalid: {source}")]
    TlsOptions {
        #[source]
        source: std::io::Error,
    },

    #[error("local mode requires SYND_LOCAL_TOKEN")]
    LocalTokenRequired,

    #[error("local token must not be empty")]
    EmptyLocalToken,
}
