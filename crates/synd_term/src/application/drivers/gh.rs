use futures_util::FutureExt as _;

use crate::{
    application::{
        Generation, Populate, RequestError, RequestKind,
        outbound::gh::{GhClient, GhError},
    },
    client::gh::FetchNotificationsParams,
    event::{Event, GhEvent},
    types::gh::{
        IssueId, NotificationContext, NotificationDetail, NotificationId, PullRequestId, ThreadId,
    },
};

use super::request::{RequestContext, RequestFuture};

/// Executes GitHub notification API requests.
pub(super) struct GhDriver {
    client: GhClientState,
}

impl GhDriver {
    pub(super) fn new(client: Option<GhClient>) -> Self {
        Self {
            client: GhClientState::from(client),
        }
    }

    pub(super) fn fetch_notifications(
        &self,
        context: RequestContext,
        generation: Generation,
        populate: Populate,
        params: FetchNotificationsParams,
    ) -> (RequestKind, RequestFuture) {
        let client = self.client.clone();

        (
            RequestKind::FetchGhNotifications {
                generation,
                page: params.page,
            },
            async move {
                let notifications = client
                    .resolve()
                    .map_err(RequestError::Gh)?
                    .fetch_notifications(params)
                    .await
                    .map_err(RequestError::Gh)?;
                context.emit(|request_id| Event::Gh {
                    request_id,
                    event: GhEvent::NotificationsFetched {
                        generation,
                        populate,
                        notifications,
                    },
                });
                Ok(())
            }
            .boxed(),
        )
    }

    pub(super) fn fetch_detail(
        &self,
        context: RequestContext,
        generation: Generation,
        detail: NotificationDetail,
    ) -> (RequestKind, RequestFuture) {
        match detail {
            NotificationDetail::Issue(issue) => self.fetch_issue(context, generation, issue),
            NotificationDetail::PullRequest(pr) => self.fetch_pull_request(context, generation, pr),
        }
    }

    fn fetch_issue(
        &self,
        context: RequestContext,
        generation: Generation,
        issue: NotificationContext<IssueId>,
    ) -> (RequestKind, RequestFuture) {
        let client = self.client.clone();

        (
            RequestKind::FetchGhIssue {
                generation,
                id: issue.id,
            },
            async move {
                let notification_id = issue.notification_id;
                let issue = client
                    .resolve()
                    .map_err(RequestError::Gh)?
                    .fetch_issue(issue)
                    .await
                    .map_err(RequestError::Gh)?;
                context.emit(|request_id| Event::Gh {
                    request_id,
                    event: GhEvent::IssueFetched {
                        generation,
                        notification_id,
                        issue,
                    },
                });
                Ok(())
            }
            .boxed(),
        )
    }

    fn fetch_pull_request(
        &self,
        context: RequestContext,
        generation: Generation,
        pull_request: NotificationContext<PullRequestId>,
    ) -> (RequestKind, RequestFuture) {
        let client = self.client.clone();

        (
            RequestKind::FetchGhPullRequest {
                generation,
                id: pull_request.id,
            },
            async move {
                let notification_id = pull_request.notification_id;
                let pull_request = client
                    .resolve()
                    .map_err(RequestError::Gh)?
                    .fetch_pull_request(pull_request)
                    .await
                    .map_err(RequestError::Gh)?;
                context.emit(|request_id| Event::Gh {
                    request_id,
                    event: GhEvent::PullRequestFetched {
                        generation,
                        notification_id,
                        pull_request,
                    },
                });
                Ok(())
            }
            .boxed(),
        )
    }

    pub(super) fn mark_notification_as_done(
        &self,
        context: RequestContext,
        id: NotificationId,
    ) -> (RequestKind, RequestFuture) {
        let client = self.client.clone();

        (
            RequestKind::MarkGhNotificationAsDone { id },
            async move {
                client
                    .resolve()
                    .map_err(RequestError::Gh)?
                    .mark_thread_as_done(id)
                    .await
                    .map_err(RequestError::Gh)?;
                context.emit(|request_id| Event::Gh {
                    request_id,
                    event: GhEvent::NotificationMarkedAsDone {
                        notification_id: id,
                    },
                });
                Ok(())
            }
            .boxed(),
        )
    }

    pub(super) fn unsubscribe_thread(
        &self,
        _context: RequestContext,
        id: ThreadId,
    ) -> (RequestKind, RequestFuture) {
        let client = self.client.clone();

        (
            RequestKind::UnsubscribeGhThread { id },
            async move {
                client
                    .resolve()
                    .map_err(RequestError::Gh)?
                    .unsubscribe_thread(id)
                    .await
                    .map_err(RequestError::Gh)
            }
            .boxed(),
        )
    }
}

/// GitHub API configuration cloned into each request.
#[derive(Clone)]
enum GhClientState {
    Configured(GhClient),
    NotConfigured,
}

impl GhClientState {
    fn resolve(self) -> Result<GhClient, GhError> {
        match self {
            Self::Configured(client) => Ok(client),
            Self::NotConfigured => Err(GhError::NotConfigured),
        }
    }
}

impl From<Option<GhClient>> for GhClientState {
    fn from(client: Option<GhClient>) -> Self {
        match client {
            Some(client) => Self::Configured(client),
            None => Self::NotConfigured,
        }
    }
}
