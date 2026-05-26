use crate::auth::{Credential, Verified};

use super::Application;

impl Application {
    pub(super) fn complete_device_authorize_flow(&mut self, cred: Verified<Credential>) {
        if let Err(err) = self.drivers.persist_credential(&cred) {
            tracing::error!("Failed to save credential to cache: {err}");
        }

        self.handle_restored_credential(cred);
    }
}
