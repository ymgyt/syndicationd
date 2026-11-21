use std::{future::Future, sync::Arc};

use graphql_client::GraphQLQuery;
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Semaphore;
use tracing::{debug, error, instrument};

use crate::{
    config,
    types::gh::{
        IssueContext, IssueId, Notification, NotificationContext, NotificationId,
        PullRequestContext, PullRequestId, RepositoryKey, ThreadId,
    },
};

#[derive(Debug, Error)]
pub enum GhError {
    #[error("GitHub client is not configured")]
    NotConfigured,
    #[error("invalid credential. please make sure a valid PAT is set")]
    BadCredential,
    // https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api?apiVersion=2022-11-28#about-secondary-rate-limits
    #[error("secondary rate limits exceeded")]
    SecondaryRateLimit,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("GraphQL: {0:?}")]
    Graphql(Vec<graphql_client::Error>),
    #[error("unexpected GitHub GraphQL response")]
    UnexpectedResponse,
    #[error("GitHub API error: {0}")]
    Api(Box<octocrab::Error>),
}

impl From<octocrab::Error> for GhError {
    fn from(err: octocrab::Error) -> Self {
        match &err {
            octocrab::Error::GitHub { source, .. } => match source.status_code.as_u16() {
                401 => GhError::BadCredential,
                403 if source.message.contains("secondary rate limit") => {
                    GhError::SecondaryRateLimit
                }
                _ => GhError::Api(Box::new(err)),
            },
            _ => GhError::Api(Box::new(err)),
        }
    }
}

impl From<Vec<graphql_client::Error>> for GhError {
    fn from(err: Vec<graphql_client::Error>) -> Self {
        GhError::Graphql(err)
    }
}

#[derive(Clone)]
pub struct GhClient {
    client: Octocrab,
    request_slots: Arc<Semaphore>,
}

// Leave headroom below GitHub's secondary limit for concurrent REST and GraphQL requests.
const MAX_CONCURRENT_REQUESTS: usize = 90;

impl GhClient {
    pub fn new(pat: impl Into<String>) -> Result<Self, GhError> {
        let pat = pat.into();
        if pat.is_empty() {
            return Err(GhError::BadCredential);
        }
        let timeout = Some(config::gh::CLIENT_TIMEOUT);
        let octo = Octocrab::builder()
            .personal_token(pat)
            .set_connect_timeout(timeout)
            .set_read_timeout(timeout)
            .set_write_timeout(timeout)
            .build()
            .unwrap();
        Ok(Self::with(octo))
    }

    #[must_use]
    pub fn with(client: Octocrab) -> Self {
        Self {
            client,
            request_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)),
        }
    }

    async fn request<T>(
        &self,
        request: impl Future<Output = octocrab::Result<T>>,
    ) -> Result<T, GhError> {
        let _permit = self
            .request_slots
            .acquire()
            .await
            .expect("GitHub request semaphore remains open");
        Ok(request.await?)
    }

    pub(crate) async fn mark_thread_as_done(&self, id: NotificationId) -> Result<(), GhError> {
        self.request(self.client.activity().notifications().mark_as_read(id))
            .await
    }

    pub(crate) async fn unsubscribe_thread(&self, id: ThreadId) -> Result<(), GhError> {
        // The reasons for not using the `set_thread_subscription` method of `NotificationHandler` are twofold:
        // 1. Since the API require the PUT method, but it is implemented using GET, it results in a "Not found" error.
        // 2. During the deserialization of the `ThreadSubscription` response type, an empty string is assigned to the reason, causing an error when deserializing the `Reason` enum.
        // https://github.com/XAMPPRocky/octocrab/pull/661

        #[derive(serde::Serialize)]
        struct Inner {
            ignored: bool,
        }
        #[derive(serde::Deserialize)]
        struct Response {}

        let thread = id;
        let ignored = true;

        let route = format!("/notifications/threads/{thread}/subscription");
        let body = Inner { ignored };

        self.request(self.client.put::<Response, _, _>(route, Some(&body)))
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum FetchNotificationInclude {
    /// Fetch only unread notifications
    OnlyUnread,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum FetchNotificationParticipating {
    /// Fetch only participating notifications
    OnlyParticipating,
    All,
}

#[derive(Debug, Clone)]
pub(crate) struct FetchNotificationsParams {
    pub(crate) page: u8,
    pub(crate) include: FetchNotificationInclude,
    pub(crate) participating: FetchNotificationParticipating,
}

impl GhClient {
    #[instrument(skip(self))]
    pub(crate) async fn fetch_notifications(
        &self,
        FetchNotificationsParams {
            page,
            include,
            participating,
        }: FetchNotificationsParams,
    ) -> Result<Vec<Notification>, GhError> {
        let mut page = self
            .request(
                self.client
                    .activity()
                    .notifications()
                    .list()
                    .participating(
                        participating == FetchNotificationParticipating::OnlyParticipating,
                    )
                    .all(include == FetchNotificationInclude::All)
                    .page(page) // 1 Origin
                    .per_page(config::gh::NOTIFICATION_PER_PAGE)
                    .send(),
            )
            .await?;
        let notifications: Vec<_> = page
            .take_items()
            .into_iter()
            .map(Notification::from)
            .collect();

        debug!(
            "Fetch {} GitHub notifications: {page:?}",
            notifications.len()
        );

        Ok(notifications)
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/client/github/schema.json",
    query_path = "src/client/github/issue_query.gql",
    variables_derives = "Clone, Debug",
    response_derives = "Clone, Debug"
)]
pub(crate) struct IssueQuery;

impl GhClient {
    pub(crate) async fn fetch_issue(
        &self,
        NotificationContext {
            id,
            repository_key: RepositoryKey { name, owner },
            ..
        }: NotificationContext<IssueId>,
    ) -> Result<IssueContext, GhError> {
        let query = IssueQuery::build_query(issue_query::Variables {
            repository_owner: owner,
            repository_name: name,
            issue_number: id.into_inner(),
        });
        let response: issue_query::ResponseData = self.request(self.client.graphql(&query)).await?;

        IssueContext::try_from(response).map_err(|error| {
            error!(%error, "failed to decode GitHub issue response");
            err::handle_decode_error(error)
        })
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/client/github/schema.json",
    query_path = "src/client/github/pull_request_query.gql",
    variables_derives = "Clone, Debug",
    response_derives = "Clone, Debug"
)]
pub(crate) struct PullRequestQuery;

impl GhClient {
    pub(crate) async fn fetch_pull_request(
        &self,
        NotificationContext {
            id,
            repository_key: RepositoryKey { name, owner },
            ..
        }: NotificationContext<PullRequestId>,
    ) -> Result<PullRequestContext, GhError> {
        let query = PullRequestQuery::build_query(pull_request_query::Variables {
            repository_owner: owner,
            repository_name: name,
            pull_request_number: id.into_inner(),
        });
        let response: pull_request_query::ResponseData =
            self.request(self.client.graphql(&query)).await?;

        PullRequestContext::try_from(response).map_err(|error| {
            error!(%error, "failed to decode GitHub pull request response");
            err::handle_decode_error(error)
        })
    }
}

mod err {
    use crate::{client::gh::GhError, types::gh::SubjectContextDecodeError};

    pub(super) fn handle_decode_error(error: SubjectContextDecodeError) -> GhError {
        match error {
            SubjectContextDecodeError::Repository => {
                GhError::NotFound("repository not found in GraphQL response".to_owned())
            }
            SubjectContextDecodeError::Issue => {
                GhError::NotFound("issue not found in GraphQL response".to_owned())
            }
            SubjectContextDecodeError::PullRequest => {
                GhError::NotFound("pull request not found in GraphQL response".to_owned())
            }
        }
    }
}
