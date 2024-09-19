use std::fmt;

use synd_kvsd_protocol::{Key, Value};

use crate::uow::Work;

pub(crate) struct GetWork(Work<GetRequest, Option<Value>>);

pub struct GetRequest {
    pub namespace: String,
    pub table: String,
    pub key: Key,
}

impl fmt::Display for GetRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Get {}/{} {}", self.namespace, self.table, self.key,)
    }
}
