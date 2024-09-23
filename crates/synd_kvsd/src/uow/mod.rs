// TODO: remove
#![expect(dead_code)]

mod set;
pub(crate) use set::SetWork;
mod get;
pub(crate) use get::GetWork;
mod delete;
pub(crate) use delete::DeleteWork;
mod authenticate;
pub(crate) use authenticate::AuthenticateWork;
mod ping;
pub(crate) use ping::PingWork;
use thiserror::Error;
mod channel;
pub(crate) use channel::{UowChannel, UowReceiver, UowSender};

use std::sync::Arc;

use tokio::sync::oneshot;

use crate::authn::principal::Principal;

#[derive(Error, Debug)]
pub(crate) enum UowError {
    #[error("send response to channel")]
    SendResponse,
}

pub(crate) enum UnitOfWork {
    Authenticate(AuthenticateWork),
    Ping(PingWork),
    Set(SetWork),
    Get(GetWork),
    Delete(DeleteWork),
}

impl UnitOfWork {
    pub(crate) fn channel(buffer: usize) -> UowChannel {
        UowChannel::new(buffer)
    }
}

pub(crate) struct Work<Req, Res> {
    pub(crate) principal: Arc<Principal>,
    pub(crate) request: Req,
    // TODO: try remove Option
    // Wrap with option so that response can be sent via mut reference.
    pub(crate) response_sender: Option<oneshot::Sender<Result<Res, UowError>>>,
}

impl<Req, Res> Work<Req, Res> {
    fn new(principal: Principal, request: Req) -> (Self, oneshot::Receiver<Result<Res, UowError>>) {
        let (tx, rx) = oneshot::channel();

        (
            Self {
                principal: Arc::new(principal),
                request,
                response_sender: Some(tx),
            },
            rx,
        )
    }

    pub(crate) fn send_response(
        &mut self,
        // TODO: use differente error
        response: Result<Res, UowError>,
    ) -> Result<(), UowError> {
        self.response_sender
            .take()
            .expect("response already sent")
            .send(response)
            // TODO: encode failed response
            .map_err(|_| UowError::SendResponse)
    }
}

/*
impl UnitOfWork {
    pub(crate) fn new_ping(
        principal: Arc<Principal>,
    ) -> (UnitOfWork, oneshot::Receiver<Result<Time>>) {
        let (tx, rx) = oneshot::channel();
        (
            UnitOfWork::Ping(Work {
                principal,
                request: (),
                response_sender: Some(tx),
            }),
            rx,
        )
    }

    pub(crate) fn new_set(
        principal: Arc<Principal>,
        set: Set,
    ) -> (UnitOfWork, oneshot::Receiver<Result<Option<Value>>>) {
        let (tx, rx) = oneshot::channel();
        (
            UnitOfWork::Set(Work {
                principal,
                request: set,
                response_sender: Some(tx),
            }),
            rx,
        )
    }

    pub(crate) fn new_get(
        principal: Arc<Principal>,
        get: Get,
    ) -> (UnitOfWork, oneshot::Receiver<Result<Option<Value>>>) {
        let (tx, rx) = oneshot::channel();
        (
            UnitOfWork::Get(Work {
                principal,
                request: get,
                response_sender: Some(tx),
            }),
            rx,
        )
    }

    pub(crate) fn new_delete(
        principal: Arc<Principal>,
        delete: Delete,
    ) -> (UnitOfWork, oneshot::Receiver<Result<Option<Value>>>) {
        let (tx, rx) = oneshot::channel();
        (
            UnitOfWork::Delete(Work {
                principal,
                request: delete,
                response_sender: Some(tx),
            }),
            rx,
        )
    }
}

impl fmt::Debug for UnitOfWork {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnitOfWork::Authenticate(_) => {
                write!(f, "Authenticate")
            }
            UnitOfWork::Ping(_) => {
                write!(f, "Ping")
            }
            UnitOfWork::Set(set) => {
                write!(f, "{}", set.request)
            }
            UnitOfWork::Get(get) => {
                write!(f, "{}", get.request)
            }
            UnitOfWork::Delete(delete) => {
                write!(f, "{}", delete.request)
            }
        }
    }
}
*/
