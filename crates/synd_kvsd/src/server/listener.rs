use std::sync::Arc;

use tokio::{
    net::{TcpListener, TcpStream},
    sync::Semaphore,
};

use crate::server::{IncommingConnection, ServerError};

pub(super) struct ConcurrencyLimited<Listener> {
    listener: Listener,
    semaphore: Arc<Semaphore>,
}

impl<Listener> ConcurrencyLimited<Listener> {
    pub(super) fn new(listener: Listener, max_connections: usize) -> Self {
        Self {
            listener,
            semaphore: Arc::new(Semaphore::new(max_connections)),
        }
    }
}

impl ConcurrencyLimited<TcpListener> {
    pub(super) async fn accept(&mut self) -> Result<IncommingConnection<TcpStream>, ServerError> {
        let permit = self.semaphore.clone().acquire_owned().await?;
        let (stream, peer_addr) = self.listener.accept().await.map_err(ServerError::accept)?;

        Ok(IncommingConnection {
            connection: stream,
            peer_addr,
            permit,
        })
    }
}
