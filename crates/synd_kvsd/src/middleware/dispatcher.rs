use std::collections::HashMap;

use thiserror::Error;

use crate::{
    middleware::Middleware,
    table::{Namespace, TableRef},
    uow::{UnitOfWork, UowSender},
};

#[derive(Error, Debug)]
pub(crate) enum DispatchError {
    #[error("table not found")]
    TableNotFound,
}

pub(crate) struct Dispatcher {
    // TODO: use TableName
    table: HashMap<Namespace, HashMap<String, UowSender>>,
}

impl Dispatcher {
    pub(crate) fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub(crate) fn add_table(&mut self, table_ref: TableRef<'_>, sender: UowSender) {
        self.table
            .entry(table_ref.namespace)
            .or_default()
            .insert(table_ref.name.into(), sender);
    }

    /*
    fn lookup_table(&self, namespace: &str, table: &str) -> Result<&mpsc::Sender<UnitOfWork>> {
        self.table
            .get(namespace)
            .and_then(|tables| tables.get(table))
            .ok_or_else(|| ErrorKind::TableNotFound(format!("{}/{}", namespace, table)).into())
    }
    */
}

impl Middleware for Dispatcher {
    type Error = DispatchError;

    async fn handle(&mut self, _uow: UnitOfWork) -> Result<(), Self::Error> {
        // TODO: dispatch
        Err(DispatchError::TableNotFound)
    }
}
