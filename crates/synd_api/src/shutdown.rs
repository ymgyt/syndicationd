use tokio::sync::broadcast::{self, Receiver, Sender};

pub struct Shutdown {
    tx: Sender<()>,
    rx: Receiver<()>,
}

impl Shutdown {
    pub fn watch_signal() -> Self {
        let (tx, rx) = broadcast::channel(2);

        let tx2 = tx.clone();
        tokio::spawn(async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => tracing::info!("Received ctrl-c signal"),
                Err(err) => tracing::error!("Failed to handle signal {err}"),
            }
            tx2.send(()).ok();
        });

        Self { tx, rx }
    }

    pub async fn notify(mut self) {
        self.rx.recv().await.ok();
    }
}

impl Clone for Shutdown {
    fn clone(&self) -> Self {
        let rx = self.tx.subscribe();
        let tx = self.tx.clone();
        Self { tx, rx }
    }
}
