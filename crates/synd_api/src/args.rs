use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, propagate_version = true, disable_help_subcommand = true)]
pub struct Args {
    #[command(flatten)]
    pub kvsd: KvsdOptions,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "kvsd")]
pub struct KvsdOptions {
    #[arg(long = "kvsd-host")]
    pub host: String,
    #[arg(long = "kvsd-port")]
    pub port: u16,
    #[arg(long = "kvsd-username")]
    pub username: String,
    #[arg(long = "kvsd-password")]
    pub password: String,
}

#[must_use]
pub fn parse() -> Args {
    Args::parse()
}
