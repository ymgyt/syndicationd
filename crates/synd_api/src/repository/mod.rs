mod subscription;
use ::kvsd::KvsdError;
pub use subscription::SubscriptionRepository;

pub mod kvsd;
pub mod types;

#[derive(thiserror::Error, Debug)]
pub enum RepositoryError {
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
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
