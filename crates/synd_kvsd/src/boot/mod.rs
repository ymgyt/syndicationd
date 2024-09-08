use std::path::PathBuf;

use thiserror::Error;

use crate::boot::provision::{ProvisionError, Provisioner};

mod provision;

#[derive(Error, Debug)]
pub enum BootError {
    #[error("failed to provision: {0}")]
    Provision(#[from] ProvisionError),
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

    pub fn boot(self) -> Result<(), BootError> {
        let prov = Provisioner::new(self.root_dir);

        prov.provision()?;
        // Walk table direcotries
        // Create Dispatcher
        // Create Middleware
        // Create Kvsd
        Ok(())
    }
}
