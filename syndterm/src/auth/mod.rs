use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::{
    args::{self, LoginCommand, LoginProtocol},
    config,
};

mod device_flow;
mod github;

#[derive(Serialize, Deserialize)]
pub enum Authentication {
    Github { access_token: String },
}

pub async fn login(cmd: LoginCommand) {
    let result = match cmd.protocol {
        LoginProtocol::OAuth(oauth) => match oauth.authorization_server {
            args::AuthorizationServer::Github => github::DeviceFlow::new()
                .device_flow(std::io::stdout())
                .await
                .map(|res| Authentication::Github {
                    access_token: res.access_token,
                }),
        },
    };

    let exit_code = match result.and_then(persist_authentication) {
        Ok(_) => {
            info!("Successfully logined");
            0
        }
        Err(err) => {
            error!("{err}");
            1
        }
    };

    std::process::exit(exit_code);
}

fn persist_authentication(auth: Authentication) -> anyhow::Result<()> {
    let auth_path = auth_file();
    if let Some(parent) = auth_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut auth_file = std::fs::File::create(&auth_path)?;

    debug!(path = ?auth_path.display(), "Create auth cache file");

    serde_json::to_writer(&mut auth_file, &auth)?;

    Ok(())
}

fn auth_file() -> PathBuf {
    config::cache_dir().join("auth.json")
}

pub fn authenticate_from_cache() -> Option<Authentication> {
    match std::fs::File::open(auth_file()) {
        Ok(f) => serde_json::from_reader(&f).ok(),
        Err(_) => None,
    }
}
