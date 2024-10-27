use tokio::io::AsyncWriteExt;

use crate::message::spec;

pub(crate) trait MessageWriteExt: AsyncWriteExt + Unpin {
    async fn write_u64m(&mut self, val: u64) -> std::io::Result<()> {
        use std::io::Write;

        // for write u64::MAX
        let mut buf = [0u8; 20];
        let mut buf = std::io::Cursor::new(&mut buf[..]);
        write!(&mut buf, "{val}")?;

        let pos: usize = buf.position().try_into().unwrap();
        self.write_all(&buf.get_ref()[..pos]).await?;
        self.write_all(spec::DELIMITER).await
    }
}

impl<T> MessageWriteExt for T where T: AsyncWriteExt + Unpin {}
