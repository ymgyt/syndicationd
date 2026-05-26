use futures_util::FutureExt;
use update_informer::{Check, registry};

use crate::event::Event;

use super::DriverContext;

pub(super) struct ReleaseDriver;

impl ReleaseDriver {
    pub(super) fn check_latest_release(cx: &mut DriverContext<'_>) -> Vec<Event> {
        let check = tokio::task::spawn_blocking(|| {
            let name = env!("CARGO_PKG_NAME");
            let version = env!("CARGO_PKG_VERSION");
            #[cfg(not(test))]
            let informer = update_informer::new(registry::Crates, name, version)
                .interval(std::time::Duration::from_hours(24))
                .timeout(std::time::Duration::from_secs(5));

            #[cfg(test)]
            let informer = update_informer::fake(registry::Crates, name, version, "v1.0.0");

            informer.check_version().ok().flatten()
        });
        let fut = async move {
            match check.await {
                Ok(Some(version)) => Ok(Event::LatestReleaseFound(version)),
                _ => Ok(Event::Nop),
            }
        }
        .boxed();
        cx.runtime.push_job(fut);
        Vec::new()
    }
}
