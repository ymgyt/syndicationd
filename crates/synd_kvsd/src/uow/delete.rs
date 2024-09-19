use std::fmt;

use synd_kvsd_protocol::{Key, Value};

use crate::uow::Work;

pub(crate) struct DeleteWork(Work<DeleteRequest, Option<Value>>);

pub struct DeleteRequest {
    pub namespace: String,
    pub table: String,
    pub key: Key,
}

impl fmt::Display for DeleteRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Delete {}/{} {}", self.namespace, self.table, self.key,)
    }
}
