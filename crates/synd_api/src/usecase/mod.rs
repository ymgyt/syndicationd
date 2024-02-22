mod subscribe_feed;
pub use subscribe_feed::{
    SubscribeFeed, SubscribeFeedError, SubscribeFeedInput, SubscribeFeedOutput,
};

mod unsubscribe_feed;
pub use unsubscribe_feed::{UnsubscribeFeed, UnsubscribeFeedInput, UnsubscribeFeedOutput};

mod fetch_subscribed_feeds;
pub use fetch_subscribed_feeds::{
    FetchSubscribedFeeds, FetchSubscribedFeedsError, FetchSubscribedFeedsInput,
    FetchSubscribedFeedsOutput,
};

mod fetch_entries;
pub use fetch_entries::{FetchEntries, FetchEntriesError, FetchEntriesInput, FetchEntriesOutput};

use tracing::error;

pub mod authorize;
use std::{future::Future, sync::Arc};

use synd_feed::feed::cache::FetchCachedFeed;
use synd_o11y::{audit, metric, tracing_subscriber::audit::Audit};

use crate::{
    principal::Principal,
    repository::{RepositoryError, SubscriptionRepository},
};

use self::authorize::{Authorized, Authorizer, Unauthorized};

pub struct MakeUsecase {
    pub subscription_repo: Arc<dyn SubscriptionRepository>,
    pub fetch_feed: Arc<dyn FetchCachedFeed>,
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
    #[error("unauthorized error")]
    Unauthorized(Unauthorized),
    #[error("repository error")]
    Repository(#[from] RepositoryError),
}

pub trait Usecase {
    type Input;
    type Output;
    type Error: std::fmt::Debug;

    fn new(make: &MakeUsecase) -> Self;

    fn audit_operation(&self) -> &'static str {
        let name = std::any::type_name::<Self>();
        // extract last element
        name.split("::").last().unwrap_or("?")
    }

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

pub struct Runtime {
    make_usecase: MakeUsecase,
    authorizer: Authorizer,
}

impl Runtime {
    pub fn new(make: MakeUsecase, authorizer: Authorizer) -> Self {
        Self {
            make_usecase: make,
            authorizer,
        }
    }

    pub async fn run<Uc, Cx, In>(
        &self,
        cx: Cx,
        input: In,
    ) -> Result<Output<Uc::Output>, Error<Uc::Error>>
    where
        Uc: Usecase + Sync + Send,
        Cx: Context,
        In: Into<Uc::Input>,
    {
        let principal = cx.principal();
        let uc = self.make_usecase.make::<Uc>();
        let input = input.into();

        {
            let user_id = principal.user_id().unwrap_or("?");
            let operation = uc.audit_operation();
            audit!(
                { Audit::USER_ID } = user_id,
                { Audit::OPERATION } = operation,
            );
            metric!(monotonic_counter.usecase = 1, operation);
        }

        let principal = match self.authorizer.authorize(principal, &uc, &input).await {
            Ok(authorized_principal) => authorized_principal,
            Err(unauthorized) => {
                audit!({ Audit::RESULT } = "unauthorized");
                return Err(Error::Unauthorized(unauthorized));
            }
        };

        let input = Input { principal, input };

        match uc.usecase(input).await {
            Ok(output) => {
                audit!({ Audit::RESULT } = "success");
                Ok(output)
            }
            Err(err) => {
                // TODO: match or method
                audit!({ Audit::RESULT } = "error");
                error!("{err:?}");
                Err(err)
            }
        }
    }
}

pub trait Context {
    fn principal(&self) -> Principal;
}
