mod datastore;
pub use datastore::Datastore;

pub mod kvsd;
pub mod memory;
pub mod types;

#[derive(thiserror::Error, Debug)]
pub enum DatastoreError {
    #[error("internal error")]
    Internal,
}
