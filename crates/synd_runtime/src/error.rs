use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Internal(#[from] anyhow::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    RegistryDb(#[from] synd_registry::RegistryDbError),

    #[error(transparent)]
    Api(#[from] synd_client::SyndApiError),

    #[error("{0} is not implemented yet")]
    NotImplemented(&'static str),
}
