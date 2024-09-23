mod handler;
mod listener;

use std::{io, net::SocketAddr};

use futures_util::TryFutureExt;
use synd_kvsd_protocol::Connection;
use thiserror::Error;
use tokio::{
    net::TcpListener,
    sync::{AcquireError, OwnedSemaphorePermit},
};

use crate::{
    server::{handler::Handler, listener::ConcurrencyLimited},
    uow::UowSender,
};

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("bind: {source}")]
    Bind { source: io::Error },
    #[error("acuire: {0}")]
    AcquireSemahprePermit(#[from] AcquireError),
    #[error("accept: {source}")]
    Accept { source: io::Error },
}

impl ServerError {
    fn bind(source: io::Error) -> Self {
        ServerError::Bind { source }
    }

    fn accept(source: io::Error) -> Self {
        ServerError::Accept { source }
    }
}

struct IncommingConnection<Connection> {
    connection: Connection,
    peer_addr: SocketAddr,
    permit: OwnedSemaphorePermit,
}

impl<Connection> IncommingConnection<Connection> {
    fn map<T, F>(self, f: F) -> IncommingConnection<T>
    where
        F: FnOnce(Connection) -> T,
    {
        IncommingConnection {
            connection: f(self.connection),
            peer_addr: self.peer_addr,
            permit: self.permit,
        }
    }
}

pub struct ServerConfig {
    pub(crate) max_connections: usize,
    pub(crate) connection_buffer: usize,
}

pub struct Server {
    config: ServerConfig,
    sender: UowSender,
}

impl Server {
    pub fn new(config: ServerConfig, sender: UowSender) -> Self {
        Self { config, sender }
    }

    pub async fn listen_and_serve(self, addr: SocketAddr) -> Result<(), ServerError> {
        TcpListener::bind(addr)
            .map_err(ServerError::bind)
            .and_then(|listener| self.serve(listener))
            .await
    }

    pub async fn serve(self, listener: TcpListener) -> Result<(), ServerError> {
        let Server {
            config:
                ServerConfig {
                    max_connections,
                    connection_buffer,
                },
            sender,
        } = self;

        let mut listener = ConcurrencyLimited::new(listener, max_connections);

        loop {
            let connection = listener
                .accept()
                .await?
                .map(|stream| Connection::new(stream, connection_buffer));
            let handler = Handler::new(connection, sender.clone());
            tokio::spawn(handler.handle());
        }
    }
}
