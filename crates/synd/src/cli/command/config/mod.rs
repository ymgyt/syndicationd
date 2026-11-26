use clap::{Args, Subcommand};

use crate::config::ConfigResolver;

mod init;
mod view;

/// Manage configurations
#[derive(Args, Debug)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum ConfigSubcommand {
    Init(init::ConfigInitCommand),
    #[command(visible_alias = "show")]
    View(view::ConfigViewCommand),
}

impl ConfigCommand {
    pub fn run(self, config: &ConfigResolver) -> anyhow::Result<()> {
        match self.command {
            ConfigSubcommand::Init(init) => init.run(),
            ConfigSubcommand::View(view) => view.run(config),
        }
    }
}
