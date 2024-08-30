use std::process::ExitCode;

use clap::Args;

use crate::config;

/// Print configuration template
#[derive(Args, Debug)]
pub struct ConfigInitCommand {}

impl ConfigInitCommand {
    #[allow(clippy::unused_self)]
    pub fn run(self) -> ExitCode {
        print!("{}", config::INIT_CONFIG.trim_start().trim_end());
        ExitCode::SUCCESS
    }
}
