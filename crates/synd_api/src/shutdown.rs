use std::{future::Future, io, net::SocketAddr, time::Duration};

use axum_server::Handle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

/// `CancellationToken` wrapper
#[derive(Clone)]
pub struct Shutdown {
    root: CancellationToken,
    handle: Handle<SocketAddr>,
}

impl Shutdown {
    pub fn manual<F>(on_graceful_shutdown: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let root = CancellationToken::new();

        let ct = root.clone();
        let handle = axum_server::Handle::new();
        let notify = handle.clone();
        tokio::spawn(async move {
            ct.cancelled().await;
            on_graceful_shutdown();
            info!("Notify axum handler to shutdown");
            notify.graceful_shutdown(Some(Duration::from_secs(3)));
        });

        Self { root, handle }
    }

    /// When the given signal Future is resolved, call the `cancel` method of the held `CancellationToken`.
    pub fn watch_signal<Fut, F>(signal: Fut, on_graceful_shutdown: F) -> Self
    where
        F: FnOnce() + Send + 'static,
        Fut: Future<Output = io::Result<()>> + Send + 'static,
    {
        let shutdown = Self::manual(on_graceful_shutdown);
        let notify = shutdown.root.clone();
        tokio::spawn(async move {
            match signal.await {
                Ok(()) => info!("Received signal"),

                Err(err) => error!("Failed to handle signal {err}"),
            }
            notify.cancel();
        });

        shutdown
    }

    /// Request shutdown
    pub fn shutdown(&self) {
        self.root.cancel();
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.root.is_cancelled()
    }

    pub fn into_handle(self) -> Handle<SocketAddr> {
        self.handle
    }

    /// Return `CancellationToken which is cancelled at shutdown`
    pub fn cancellation_token(&self) -> CancellationToken {
        self.root.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::ErrorKind,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use futures_util::future;

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn signal_trigger_graceful_shutdown() {
        for signal_result in [Ok(()), Err(io::Error::from(ErrorKind::Other))] {
            let called = Arc::new(AtomicBool::new(false));
            let called_cloned = Arc::clone(&called);
            let on_graceful_shutdown = move || {
                called_cloned.store(true, Ordering::Relaxed);
            };
            let (tx, rx) = tokio::sync::oneshot::channel::<io::Result<()>>();
            let s = Shutdown::watch_signal(
                async move {
                    rx.await.unwrap().ok();
                    signal_result
                },
                on_graceful_shutdown,
            );
            let ct = s.cancellation_token();

            // Mock signal triggered
            tx.send(Ok(())).unwrap();

            // Check cancellation token is cancelled and axum handler called
            let mut ok = false;
            let mut count = 0;
            loop {
                count += 1;
                if count >= 10 {
                    break;
                }
                if s.root.is_cancelled() && ct.is_cancelled() && called.load(Ordering::Relaxed) {
                    ok = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            assert!(ok, "cancelation does not work");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shutdown_trigger_graceful_shutdown() {
        let called = Arc::new(AtomicBool::new(false));
        let called_cloned = Arc::clone(&called);
        let on_graceful_shutdown = move || {
            called_cloned.store(true, Ordering::Relaxed);
        };
        let s = Shutdown::watch_signal(future::pending(), on_graceful_shutdown);
        let ct = s.cancellation_token();

        s.shutdown();

        // Check cancellation token is cancelled and axum handler called
        let mut ok = false;
        let mut count = 0;
        loop {
            count += 1;
            if count >= 10 {
                break;
            }
            if s.root.is_cancelled() && ct.is_cancelled() && called.load(Ordering::Relaxed) {
                ok = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(ok, "cancelation does not work");
        assert!(s.is_shutdown_requested());
    }
}
