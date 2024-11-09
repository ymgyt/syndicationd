use std::{
    io::{self},
    time::Duration,
};

use bytes::{Buf, BytesMut};
use futures::TryFutureExt as _;
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt, BufWriter},
    net::TcpStream,
    time::error::Elapsed,
};
use tracing::trace;

use crate::message::{Message, ParseError, Parser};

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("read timeout: {0}")]
    ReadTimeout(Elapsed),
    #[error("read message io: {source}")]
    ReadMessageIo { source: io::Error },
    #[error("parse message: {0}")]
    ParseMessage(#[from] ParseError),
    #[error("connection reset by peer")]
    ResetByPeer,
    #[error("write message: {source}")]
    WriteMessage { source: io::Error },
}

impl ConnectionError {
    fn read_timeout(elapsed: Elapsed) -> Self {
        ConnectionError::ReadTimeout(elapsed)
    }

    fn read_message_io(source: io::Error) -> Self {
        ConnectionError::ReadMessageIo { source }
    }

    fn write_message(source: io::Error) -> Self {
        ConnectionError::WriteMessage { source }
    }
}

pub struct Connection<Stream = TcpStream> {
    stream: BufWriter<Stream>,
    // The buffer for reading frames.
    buffer: BytesMut,
}

impl<Stream> Connection<Stream>
where
    Stream: AsyncWrite,
{
    pub fn new(stream: Stream, buffer_size: usize) -> Self {
        Self {
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(buffer_size),
        }
    }
}

impl<Stream> Connection<Stream>
where
    Stream: AsyncWrite + Unpin,
{
    pub async fn write_message(&mut self, message: Message) -> Result<(), ConnectionError> {
        message
            .write(&mut self.stream)
            .await
            .map_err(ConnectionError::write_message)?;

        self.stream
            .flush()
            .await
            .map_err(ConnectionError::write_message)
    }
}

impl<Stream> Connection<Stream>
where
    Stream: AsyncRead + Unpin,
    BufWriter<Stream>: AsyncRead,
{
    pub async fn read_message_with_timeout(
        &mut self,
        duration: Duration,
    ) -> Result<Option<Message>, ConnectionError> {
        tokio::time::timeout(duration, self.read_message())
            .map_err(ConnectionError::read_timeout)
            .await?
    }

    pub async fn read_message(&mut self) -> Result<Option<Message>, ConnectionError> {
        loop {
            let input = &self.buffer[..];
            match Parser::new().parse(input) {
                Ok((remain, message)) => {
                    let consumed = input.len() - remain.len();
                    self.buffer.advance(consumed);
                    return Ok(Some(message));
                }
                Err(ParseError::Incomplete) => {
                    let read = self
                        .stream
                        .read_buf(&mut self.buffer)
                        .await
                        .map_err(ConnectionError::read_message_io)?;
                    trace!(
                        bytes = read,
                        buffer = self.buffer.len(),
                        "read from connection"
                    );

                    if read == 0 {
                        return if self.buffer.is_empty() {
                            Ok(None)
                        } else {
                            Err(ConnectionError::ResetByPeer)
                        };
                    }
                }
                Err(err) => return Err(ConnectionError::from(err)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::message::{Authenticate, Ping};

    use super::*;

    #[tokio::test]
    async fn read_write() {
        let messages = vec![
            Message::Authenticate(Authenticate::new("user", "pass")),
            Message::Ping(Ping::new()),
        ];

        let buf_size = 1024;
        let (read, write) = tokio::io::duplex(buf_size);
        let (mut read, mut write) = (
            Connection::new(read, buf_size),
            Connection::new(write, buf_size),
        );

        for message in messages {
            write.write_message(message.clone()).await.unwrap();

            assert_eq!(read.read_message().await.unwrap(), Some(message));
        }
    }
}
