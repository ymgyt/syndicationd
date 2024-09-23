// TODO: remove
use std::path::PathBuf;

use thiserror::Error;

use crate::{
    boot::provision::{ProvisionError, Provisioner},
    kvsd::Kvsd,
    middleware::{Dispatcher, MiddlewareStack},
    table::{Table, TableRef},
    uow::UnitOfWork,
};

mod provision;

#[derive(Error, Debug)]
pub enum BootError {
    #[error("failed to provision: {0}")]
    Provision(#[from] ProvisionError),
    #[error("tablel: {message}")]
    Table { message: String },
}

pub struct Boot {
    root_dir: PathBuf,
}

impl Boot {
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
        }
    }

    pub async fn boot(self) -> Result<Kvsd, BootError> {
        let prov = Provisioner::new(self.root_dir).provision()?;
        let mut dispatcher = Dispatcher::new();

        for (namespace, table_dir) in prov.table_dirs()? {
            let table = Table::try_from_dir(table_dir)
                .await
                .map_err(|err| BootError::Table {
                    message: err.to_string(),
                })?;
            // TODO: configure buffer size
            let (tx, rx) = UnitOfWork::channel(1024).split();
            let table_ref = TableRef {
                namespace,
                name: table.name().into(),
            };
            dispatcher.add_table(table_ref, tx);

            tokio::spawn(table.run(rx));
        }

        let kvsd = {
            let mw = MiddlewareStack::new(dispatcher);
            let uow_ch = UnitOfWork::channel(1024);
            Kvsd::new(uow_ch, mw)
        };

        Ok(kvsd)
    }
}
