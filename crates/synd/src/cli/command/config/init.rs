use std::io::{self, Write as _};

use clap::Args;

use crate::config;

/// Print configuration template
#[derive(Args, Debug)]
pub struct ConfigInitCommand;

impl ConfigInitCommand {
    #[allow(clippy::unused_self)]
    pub fn run(self) -> anyhow::Result<()> {
        io::stdout()
            .lock()
            .write_all(config::INIT_CONFIG.trim().as_bytes())?;
        Ok(())
    }
}
