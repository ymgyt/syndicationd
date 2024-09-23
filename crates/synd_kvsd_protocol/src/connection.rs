use bytes::BytesMut;
use tokio::{
    io::{AsyncWrite, BufWriter},
    net::TcpStream,
};

#[expect(dead_code)]
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
