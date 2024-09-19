use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::{
    table::{Namespace, TableRef},
    uow::UnitOfWork,
};

pub(crate) struct Dispatcher {
    // TODO: use TableName
    table: HashMap<Namespace, HashMap<String, mpsc::Sender<UnitOfWork>>>,
}

impl Dispatcher {
    pub(crate) fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub(crate) fn add_table(&mut self, table_ref: TableRef<'_>, sender: mpsc::Sender<UnitOfWork>) {
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
