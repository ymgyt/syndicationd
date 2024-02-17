use std::time::Duration;

use axum_server::Handle;
use tokio::sync::broadcast::{self, Receiver, Sender};

pub struct Shutdown {
    tx: Sender<()>,
    rx: Receiver<()>,
    handle: Handle,
}

impl Shutdown {
    pub fn watch_signal() -> Self {
        let (tx, rx) = broadcast::channel(2);
        let handle = Handle::new();

        let tx2 = tx.clone();
        let handle2 = handle.clone();
        tokio::spawn(async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => tracing::info!("Received ctrl-c signal"),
                Err(err) => tracing::error!("Failed to handle signal {err}"),
            }
            // Signal graceful shutdown to axum_server
            handle2.graceful_shutdown(Some(Duration::from_secs(3)));
            tx2.send(()).ok();
        });

        Self { tx, rx, handle }
    }

    pub fn into_handle(self) -> Handle {
        self.handle
    }

    pub async fn notify(mut self) {
        self.rx.recv().await.ok();
    }
}

impl Clone for Shutdown {
    fn clone(&self) -> Self {
        let rx = self.tx.subscribe();
        let tx = self.tx.clone();
        let handle = self.handle.clone();
        Self { tx, rx, handle }
    }
}
