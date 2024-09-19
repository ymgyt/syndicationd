use tokio::sync::oneshot;

use crate::{
    authn::{credential, principal::Principal},
    uow::{UowError, Work},
};

pub(crate) struct AuthenticateWork(Work<Box<dyn credential::Provider + Send>, Option<Principal>>);

impl AuthenticateWork {
    pub(crate) fn new(
        provider: Box<dyn credential::Provider + Send>,
    ) -> (Self, oneshot::Receiver<Result<Option<Principal>, UowError>>) {
        let (work, rx) = Work::new(Principal::AnonymousUser, provider);
        (AuthenticateWork(work), rx)
    }
}
