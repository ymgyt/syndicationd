use std::{
    sync::mpsc::{self, TryRecvError},
    thread,
    time::Duration,
};

use tracing::warn;
use update_informer::{Check, Version, registry};

const RELEASE_PACKAGE: &str = env!("CARGO_PKG_NAME");
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) struct ReleaseCheck {
    receiver: mpsc::Receiver<Option<Version>>,
}

impl ReleaseCheck {
    pub(crate) fn spawn() -> Self {
        let (sender, receiver) = mpsc::channel();

        if let Err(err) = thread::Builder::new()
            .name("release-check".to_owned())
            .spawn(move || {
                let _ = sender.send(check_latest_release());
            })
        {
            warn!("Failed to spawn release check: {err}");
        }

        Self { receiver }
    }

    pub(crate) fn print_notice_if_ready(self) {
        match self.receiver.try_recv() {
            Ok(Some(new_version)) => {
                println!("A new release of synd is available: v{CURRENT_VERSION} -> {new_version}");
            }
            Ok(None) | Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
        }
    }
}

fn check_latest_release() -> Option<Version> {
    let informer = update_informer::new(registry::Crates, RELEASE_PACKAGE, CURRENT_VERSION)
        .interval(Duration::from_hours(24))
        .timeout(Duration::from_secs(5));

    informer.check_version().ok().flatten()
}
