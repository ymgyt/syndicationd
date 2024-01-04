mod datastore;
pub use datastore::Datastore;

#[derive(thiserror::Error, Debug)]
pub enum DatastoreError {
    #[error("internal error")]
    Internal,
}
