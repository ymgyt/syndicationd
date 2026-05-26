use std::time::{Duration, Instant};

use crate::client::synd_api::Client;

use super::runtime::LocalApiHandle;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) async fn wait_until_ready(
    client: &Client,
    handle: &LocalApiHandle,
) -> anyhow::Result<()> {
    let deadline = Instant::now() + STARTUP_TIMEOUT;

    loop {
        if handle.is_finished() {
            anyhow::bail!("local synd-api exited before readiness");
        }

        if client.health().await.is_ok() {
            return Ok(());
        }

        if Instant::now() >= deadline {
            anyhow::bail!("local synd-api did not become ready within {STARTUP_TIMEOUT:?}");
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
