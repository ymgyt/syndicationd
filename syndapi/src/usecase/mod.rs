pub mod subscribe_feed;

use std::{future::Future, sync::Arc};

use synd::feed::parser::FetchFeed;

use crate::{
    persistence::{Datastore, DatastoreError},
    principal::Principal,
};

use self::authorize::{Authorized, Unauthorized};

pub mod authorize;

pub struct MakeUsecase {
    pub datastore: Arc<dyn Datastore>,
    pub fetch_feed: Arc<dyn FetchFeed>,
}

impl MakeUsecase {
    pub fn make<T: Usecase + Send>(&self) -> T {
        T::new(self)
    }
}

pub struct Input<T> {
    pub principal: Authorized<Principal>,
    pub input: T,
}

pub struct Output<T> {
    pub output: T,
}

#[derive(Debug, thiserror::Error)]
pub enum Error<T> {
    #[error(transparent)]
    Usecase(T),
    #[error("datastore error")]
    Datastore(#[from] DatastoreError),
}

pub trait Usecase {
    type Input;
    type Output;
    type Error;

    fn new(make: &MakeUsecase) -> Self;

    /// Authorize given principal
    fn authorize(
        &self,
        principal: Principal,
        input: &Self::Input,
    ) -> impl Future<Output = Result<Principal, Unauthorized>>;

    /// Usecase entrypoint
    fn usecase(
        &self,
        input: Input<Self::Input>,
    ) -> impl Future<Output = Result<Output<Self::Output>, Error<Self::Error>>>;
}
