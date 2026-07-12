use std::sync::Arc;

use futures_util::FutureExt;

use crate::{
    application::outbound::github::GithubClient,
    application::{Populate, RequestId},
    client::github::FetchNotificationsParams,
    event::{ApiEvent, Event, GitHubApiEvent},
    types::github::{IssueOrPullRequest, NotificationId, ThreadId},
};

use super::runtime::DriverRuntime;

/// Executes GitHub notification API requests.
pub(super) struct GitHubDriver {
    pub(super) client: Option<GithubClient>,
}

impl GitHubDriver {
    pub(super) fn fetch_notifications(
        &self,
        runtime: &mut DriverRuntime,
        populate: Populate,
        params: FetchNotificationsParams,
    ) -> Option<Event> {
        let Some(client) = self.client.clone() else {
            return Some(Self::client_not_configured());
        };
        let request_seq =
            runtime.request_started(RequestId::FetchGithubNotifications { page: params.page });
        let fut = async move {
            match client.fetch_notifications(params).await {
                Ok(notifications) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::GitHub(GitHubApiEvent::NotificationsFetched {
                        populate,
                        notifications,
                    }),
                }),
                Err(error) => Ok(Event::GithubApiError {
                    error: Arc::new(error),
                    request_seq,
                }),
            }
        }
        .boxed();
        runtime.push_job(fut);
        None
    }

    pub(super) fn fetch_notification_details(
        &self,
        runtime: &mut DriverRuntime,
        contexts: Vec<IssueOrPullRequest>,
    ) -> Option<Event> {
        let Some(client) = self.client.clone() else {
            return Some(Self::client_not_configured());
        };

        for context in contexts {
            let client = client.clone();

            let fut = match context {
                either::Either::Left(issue) => {
                    let request_seq =
                        runtime.request_started(RequestId::FetchGithubIssue { id: issue.id });
                    let notification_id = issue.notification_id;
                    async move {
                        match client.fetch_issue(issue).await {
                            Ok(issue) => Ok(Event::Api {
                                request_seq,
                                event: ApiEvent::GitHub(GitHubApiEvent::IssueFetched {
                                    notification_id,
                                    issue,
                                }),
                            }),
                            Err(error) => Ok(Event::GithubApiError {
                                error: Arc::new(error),
                                request_seq,
                            }),
                        }
                    }
                    .boxed()
                }
                either::Either::Right(pull_request) => {
                    let request_seq = runtime.request_started(RequestId::FetchGithubPullRequest {
                        id: pull_request.id,
                    });
                    let notification_id = pull_request.notification_id;

                    async move {
                        match client.fetch_pull_request(pull_request).await {
                            Ok(pull_request) => Ok(Event::Api {
                                request_seq,
                                event: ApiEvent::GitHub(GitHubApiEvent::PullRequestFetched {
                                    notification_id,
                                    pull_request,
                                }),
                            }),
                            Err(error) => Ok(Event::GithubApiError {
                                error: Arc::new(error),
                                request_seq,
                            }),
                        }
                    }
                    .boxed()
                }
            };
            runtime.push_job(fut);
        }

        None
    }

    pub(super) fn mark_notification_as_done(
        &self,
        runtime: &mut DriverRuntime,
        id: NotificationId,
    ) -> Option<Event> {
        let Some(client) = self.client.clone() else {
            return Some(Self::client_not_configured());
        };
        let request_seq = runtime.request_started(RequestId::MarkGithubNotificationAsDone { id });
        let fut = async move {
            match client.mark_thread_as_done(id).await {
                Ok(()) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::GitHub(GitHubApiEvent::NotificationMarkedAsDone {
                        notification_id: id,
                    }),
                }),
                Err(error) => Ok(Event::GithubApiError {
                    error: Arc::new(error),
                    request_seq,
                }),
            }
        }
        .boxed();
        runtime.push_job(fut);
        None
    }

    pub(super) fn unsubscribe_thread(
        &self,
        runtime: &mut DriverRuntime,
        id: ThreadId,
    ) -> Option<Event> {
        let Some(client) = self.client.clone() else {
            return Some(Self::client_not_configured());
        };
        let request_seq = runtime.request_started(RequestId::UnsubscribeGithubThread);
        let fut = async move {
            match client.unsubscribe_thread(id).await {
                Ok(()) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::GitHub(GitHubApiEvent::ThreadUnsubscribed),
                }),
                Err(error) => Ok(Event::GithubApiError {
                    error: Arc::new(error),
                    request_seq,
                }),
            }
        }
        .boxed();
        runtime.push_job(fut);
        None
    }

    fn client_not_configured() -> Event {
        Event::Error {
            message: "github client is not configured".to_owned(),
        }
    }
}
