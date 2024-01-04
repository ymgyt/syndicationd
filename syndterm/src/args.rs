use clap::{Parser, Subcommand };
use url::Url;

use crate::config;

#[derive(Parser, Debug)]
#[command(version, propagate_version = true, about = "xxx")]
pub struct Args {
    /// syndapi endpoint
    #[arg(default_value = config::api::ENDPOINT)]
    pub endpoint: Url,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Login(LoginCommand),
    /// Clear authenticate state
    Logout,
}

/// Login to authenticate client
#[derive(clap::Args, Debug)]
pub struct LoginCommand {
    #[command(subcommand)]
    pub protocol: LoginProtocol,
}

#[derive(Subcommand, Debug)]
pub enum LoginProtocol {
    #[command(name = "oauth")]
    OAuth(OAuthLoginCommand),
}

#[derive(clap::Args,Debug)]
pub struct OAuthLoginCommand {
    /// Authorization server
    #[arg(
    value_enum, 
    long, 
    default_value_t = AuthorizationServer::Github,
    visible_alias = "auth",
    )]
    pub authorization_server: AuthorizationServer,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum AuthorizationServer {
    Github,
}

pub fn parse() -> Args {
    Args::parse()
}
