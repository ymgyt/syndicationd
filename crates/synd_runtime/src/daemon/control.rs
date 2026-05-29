use crate::{DaemonStatus, Error, Result, Runtime, ShutdownResult};

#[derive(Debug, Clone, Copy)]
pub struct Control<'a> {
    runtime: &'a Runtime,
}

impl<'a> Control<'a> {
    pub(crate) fn new(runtime: &'a Runtime) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &Runtime {
        self.runtime
    }

    #[allow(clippy::unused_async)]
    pub async fn inspect(&self) -> Result<DaemonStatus> {
        Err(Error::NotImplemented("DaemonControl::inspect"))
    }

    #[allow(clippy::unused_async)]
    pub async fn shutdown(&self) -> Result<ShutdownResult> {
        Err(Error::NotImplemented("DaemonControl::shutdown"))
    }

    #[allow(clippy::unused_async)]
    pub async fn restart(&self) -> Result<DaemonStatus> {
        Err(Error::NotImplemented("DaemonControl::restart"))
    }
}
