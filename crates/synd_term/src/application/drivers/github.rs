use std::sync::Arc;

use futures_util::FutureExt;

use crate::{
    application::{Populate, RequestId},
    client::github::FetchNotificationsParams,
    event::{ApiEvent, Event, GitHubApiEvent},
    types::github::{IssueOrPullRequest, NotificationId, ThreadId},
};

use super::DriverContext;

pub(super) struct GitHubDriver;

impl GitHubDriver {
    pub(super) fn fetch_notifications(
        cx: &mut DriverContext<'_>,
        populate: Populate,
        params: FetchNotificationsParams,
    ) -> Vec<Event> {
        let Some(client) = cx.adapters.github_client.clone() else {
            return vec![Event::Error {
                message: "github client is not configured".to_owned(),
            }];
        };
        let request_seq = cx
            .runtime
            .request_started(RequestId::FetchGithubNotifications { page: params.page });
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
        cx.runtime.push_job(fut);
        Vec::new()
    }

    pub(super) fn fetch_notification_details(
        cx: &mut DriverContext<'_>,
        contexts: Vec<IssueOrPullRequest>,
    ) -> Vec<Event> {
        let Some(client) = cx.adapters.github_client.clone() else {
            return vec![Event::Error {
                message: "github client is not configured".to_owned(),
            }];
        };

        for context in contexts {
            let client = client.clone();

            let fut = match context {
                either::Either::Left(issue) => {
                    let request_seq = cx
                        .runtime
                        .request_started(RequestId::FetchGithubIssue { id: issue.id });
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
                    let request_seq =
                        cx.runtime
                            .request_started(RequestId::FetchGithubPullRequest {
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
            cx.runtime.push_job(fut);
        }

        Vec::new()
    }

    pub(super) fn mark_notification_as_done(
        cx: &mut DriverContext<'_>,
        id: NotificationId,
    ) -> Vec<Event> {
        let Some(client) = cx.adapters.github_client.clone() else {
            return vec![Event::Error {
                message: "github client is not configured".to_owned(),
            }];
        };
        let request_seq = cx
            .runtime
            .request_started(RequestId::MarkGithubNotificationAsDone { id });
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
        cx.runtime.push_job(fut);
        Vec::new()
    }

    pub(super) fn unsubscribe_thread(cx: &mut DriverContext<'_>, id: ThreadId) -> Vec<Event> {
        let Some(client) = cx.adapters.github_client.clone() else {
            return vec![Event::Error {
                message: "github client is not configured".to_owned(),
            }];
        };
        let request_seq = cx
            .runtime
            .request_started(RequestId::UnsubscribeGithubThread);
        let fut = async move {
            match client.unsubscribe_thread(id).await {
                Ok(()) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::GitHub(GitHubApiEvent::ThreadUnsubscribed {}),
                }),
                Err(error) => Ok(Event::GithubApiError {
                    error: Arc::new(error),
                    request_seq,
                }),
            }
        }
        .boxed();
        cx.runtime.push_job(fut);
        Vec::new()
    }
}
