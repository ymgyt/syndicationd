use graphql_client::GraphQLQuery;
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    config,
    types::github::{
        IssueContext, IssueId, Notification, NotificationContext, NotificationId,
        PullRequestContext, PullRequestId, RepositoryKey, ThreadId,
    },
};

#[derive(Debug, Error)]
pub enum GithubError {
    #[error("invalid credential. please make sure a valid PAT is set")]
    BadCredential,
    // https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api?apiVersion=2022-11-28#about-secondary-rate-limits
    #[error("secondary rate limits exceeded")]
    SecondaryRateLimit,
    #[error("github api error: {0}")]
    Api(octocrab::Error),
}

impl From<octocrab::Error> for GithubError {
    fn from(err: octocrab::Error) -> Self {
        match &err {
            octocrab::Error::GitHub { source, .. } => match source.status_code.as_u16() {
                401 => GithubError::BadCredential,
                403 if source.message.contains("secondary rate limit") => {
                    GithubError::SecondaryRateLimit
                }
                _ => GithubError::Api(err),
            },
            _ => GithubError::Api(err),
        }
    }
}

#[derive(Clone)]
pub struct GithubClient {
    client: Octocrab,
}

impl GithubClient {
    pub fn new(pat: impl Into<String>) -> Result<Self, GithubError> {
        let pat = pat.into();
        if pat.is_empty() {
            return Err(GithubError::BadCredential);
        }
        let timeout = Some(config::github::CLIENT_TIMEOUT);
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
        Self { client }
    }

    pub(crate) async fn mark_thread_as_done(&self, id: NotificationId) -> Result<(), GithubError> {
        self.client
            .activity()
            .notifications()
            .mark_as_read(id)
            .await
            .map_err(GithubError::from)
    }

    pub(crate) async fn unsubscribe_thread(&self, id: ThreadId) -> Result<(), GithubError> {
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

        self.client
            .put::<Response, _, _>(route, Some(&body))
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

impl GithubClient {
    #[tracing::instrument(skip(self))]
    pub(crate) async fn fetch_notifications(
        &self,
        FetchNotificationsParams {
            page,
            include,
            participating,
        }: FetchNotificationsParams,
    ) -> Result<Vec<Notification>, GithubError> {
        let mut page = self
            .client
            .activity()
            .notifications()
            .list()
            .participating(participating == FetchNotificationParticipating::OnlyParticipating)
            .all(include == FetchNotificationInclude::All)
            .page(page) // 1 Origin
            .per_page(config::github::NOTIFICATION_PER_PAGE)
            .send()
            .await?;
        let notifications: Vec<_> = page
            .take_items()
            .into_iter()
            .map(Notification::from)
            .collect();

        tracing::debug!(
            "Fetch {} github notifications: {page:?}",
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

impl GithubClient {
    pub(crate) async fn fetch_issue(
        &self,
        NotificationContext {
            id,
            repository_key: RepositoryKey { name, owner },
            ..
        }: NotificationContext<IssueId>,
    ) -> Result<IssueContext, GithubError> {
        let response: octocrab::Result<graphql_client::Response<issue_query::ResponseData>> = self
            .client
            .graphql(&IssueQuery::build_query(issue_query::Variables {
                repository_owner: owner,
                repository_name: name,
                issue_number: id.into_inner(),
            }))
            .await;

        match response {
            Ok(response) => match (response.data, response.errors) {
                (_, Some(errors)) => {
                    tracing::error!("{errors:?}");
                    todo!()
                }
                (Some(data), _) => Ok(IssueContext::from(data)),
                _ => unreachable!(),
            },
            Err(error) => Err(GithubError::from(error)),
        }
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

impl GithubClient {
    pub(crate) async fn fetch_pull_request(
        &self,
        NotificationContext {
            id,
            repository_key: RepositoryKey { name, owner },
            ..
        }: NotificationContext<PullRequestId>,
    ) -> Result<PullRequestContext, GithubError> {
        let response: octocrab::Result<graphql_client::Response<pull_request_query::ResponseData>> =
            self.client
                .graphql(&PullRequestQuery::build_query(
                    pull_request_query::Variables {
                        repository_owner: owner,
                        repository_name: name,
                        pull_request_number: id.into_inner(),
                    },
                ))
                .await;

        match response {
            Ok(response) => match (response.data, response.errors) {
                (_, Some(errors)) => {
                    tracing::error!("{errors:?}");
                    todo!()
                }
                (Some(data), _) => Ok(PullRequestContext::from(data)),
                _ => unreachable!(),
            },
            Err(error) => Err(GithubError::from(error)),
        }
    }
}
