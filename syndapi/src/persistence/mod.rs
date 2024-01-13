mod datastore;
pub use datastore::Datastore;

pub mod kvsd;
pub mod memory;

#[derive(thiserror::Error, Debug)]
pub enum DatastoreError {
    #[error("internal error")]
    Internal,
}
