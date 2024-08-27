use std::process::ExitCode;

use clap::{Args, Subcommand};

mod init;

/// Manage configurations
#[derive(Args, Debug)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum ConfigSubcommand {
    Init(init::ConfigInitCommand),
}

impl ConfigCommand {
    pub fn run(self) -> ExitCode {
        let ConfigCommand { command } = self;

        match command {
            ConfigSubcommand::Init(init) => init.run(),
        }
    }
}
