use std::sync::atomic::{AtomicU64, Ordering};

use synd_protocol::session::SessionId;

/// Issues session IDs for one daemon run.
#[derive(Debug)]
pub struct SessionIdIssuer {
    prefix: String,
    next: AtomicU64,
}

impl SessionIdIssuer {
    /// Starts at sequence zero. Use a fresh nonce for each daemon run.
    pub fn new(pid: u32, startup_nonce: u128) -> Self {
        Self {
            prefix: format!("{pid}-{startup_nonce:032x}"),
            next: AtomicU64::new(0),
        }
    }

    pub(super) fn issue(&self) -> SessionId {
        let sequence = self.next.fetch_add(1, Ordering::Relaxed);
        SessionId::new(format!("{}/{sequence}", self.prefix))
    }
}

impl Default for SessionIdIssuer {
    fn default() -> Self {
        Self::new(std::process::id(), rand::random())
    }
}
