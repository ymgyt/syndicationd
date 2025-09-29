mod subscription;
use ::kvsd::KvsdError;
pub use subscription::SubscriptionRepository;

pub mod kvsd;
pub mod sqlite;
pub mod types;

#[derive(thiserror::Error, Debug)]
pub enum RepositoryError {
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

impl RepositoryError {
    pub fn internal(err: impl Into<anyhow::Error>) -> Self {
        RepositoryError::Internal(err.into())
    }
}

impl From<KvsdError> for RepositoryError {
    fn from(value: KvsdError) -> Self {
        RepositoryError::Internal(value.into())
    }
}

impl From<sqlx::Error> for RepositoryError {
    fn from(value: sqlx::Error) -> Self {
        RepositoryError::internal(anyhow::Error::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error() {
        assert!(
            !RepositoryError::internal(anyhow::anyhow!("error"))
                .to_string()
                .is_empty()
        );

        assert!(
            !RepositoryError::from(KvsdError::Unauthenticated)
                .to_string()
                .is_empty()
        );
    }
}
