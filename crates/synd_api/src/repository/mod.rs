mod subscription;
pub use subscription::SubscriptionRepository;

pub mod kvsd;
pub mod memory;
pub mod types;

#[derive(thiserror::Error, Debug)]
pub enum RepositoryError {
    #[error("internal error")]
    Internal,
}
