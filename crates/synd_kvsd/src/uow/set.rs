use std::fmt;

use synd_kvsd_protocol::{Key, Value};

use crate::uow::Work;

pub(crate) struct SetWork(Work<SetRequest, Option<Value>>);

pub struct SetRequest {
    // TODO: use Namespace
    pub namespace: String,
    pub table: String,
    pub key: Key,
    pub value: Value,
}

impl fmt::Display for SetRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Set {namespace}/{table} {key} => {value:?}",
            namespace = &self.namespace,
            table = &self.table,
            key = &self.key,
            value = &self.value,
        )
    }
}
