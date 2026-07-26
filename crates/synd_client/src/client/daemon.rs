use synd_protocol::daemon::DaemonStatusResponse;
use synd_support::o11y::health_check::Health;

use super::Client;
use crate::SyndApiError;

const HEALTH_CHECK_PATH: &str = "/health";
const DAEMON_STATUS_PATH: &str = synd_protocol::daemon::STATUS_PATH;
const DAEMON_SHUTDOWN_PATH: &str = "/daemon/shutdown";

impl Client {
    pub async fn health(&self) -> Result<Health, SyndApiError> {
        self.client
            .get(self.endpoint.join(HEALTH_CHECK_PATH)?)
            .send()
            .await
            .map_err(SyndApiError::from_send_error)?
            .error_for_status()
            .map_err(SyndApiError::from_status_error)?
            .json()
            .await
            .map_err(SyndApiError::DecodeResponse)
    }

    pub async fn shutdown_daemon(&self) -> Result<(), SyndApiError> {
        self.client
            .post(self.endpoint.join(DAEMON_SHUTDOWN_PATH)?)
            .send()
            .await
            .map_err(SyndApiError::from_send_error)?
            .error_for_status()
            .map_err(SyndApiError::from_status_error)?;

        Ok(())
    }

    pub async fn daemon_status(&self) -> Result<DaemonStatusResponse, SyndApiError> {
        self.client
            .get(self.endpoint.join(DAEMON_STATUS_PATH)?)
            .send()
            .await
            .map_err(SyndApiError::from_send_error)?
            .error_for_status()
            .map_err(SyndApiError::from_status_error)?
            .json()
            .await
            .map_err(SyndApiError::DecodeResponse)
    }
}
