use futures_util::FutureExt as _;

use crate::{
    application::{
        Populate, RequestError,
        outbound::gh::{GhClient, GhError},
    },
    client::gh::FetchNotificationsParams,
    event::GhEvent,
    types::gh::{IssueId, NotificationContext, NotificationId, PullRequestId, ThreadId},
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
        populate: Populate,
        params: FetchNotificationsParams,
    ) -> impl FnOnce(RequestContext) -> RequestFuture + use<> {
        let client = self.client.clone();

        move |context| {
            async move {
                let notifications = client
                    .resolve()
                    .map_err(RequestError::Gh)?
                    .fetch_notifications(params)
                    .await
                    .map_err(RequestError::Gh)?;
                context.emit_gh(GhEvent::NotificationsFetched {
                    populate,
                    notifications,
                });
                Ok(())
            }
            .boxed()
        }
    }

    pub(super) fn fetch_issue(
        &self,
        issue: NotificationContext<IssueId>,
    ) -> impl FnOnce(RequestContext) -> RequestFuture + use<> {
        let client = self.client.clone();

        move |context| {
            async move {
                let notification_id = issue.notification_id;
                let issue = client
                    .resolve()
                    .map_err(RequestError::Gh)?
                    .fetch_issue(issue)
                    .await
                    .map_err(RequestError::Gh)?;
                context.emit_gh(GhEvent::IssueFetched {
                    notification_id,
                    issue,
                });
                Ok(())
            }
            .boxed()
        }
    }

    pub(super) fn fetch_pull_request(
        &self,
        pull_request: NotificationContext<PullRequestId>,
    ) -> impl FnOnce(RequestContext) -> RequestFuture + use<> {
        let client = self.client.clone();

        move |context| {
            async move {
                let notification_id = pull_request.notification_id;
                let pull_request = client
                    .resolve()
                    .map_err(RequestError::Gh)?
                    .fetch_pull_request(pull_request)
                    .await
                    .map_err(RequestError::Gh)?;
                context.emit_gh(GhEvent::PullRequestFetched {
                    notification_id,
                    pull_request,
                });
                Ok(())
            }
            .boxed()
        }
    }

    pub(super) fn mark_notification_as_done(
        &self,
        id: NotificationId,
    ) -> impl FnOnce(RequestContext) -> RequestFuture + use<> {
        let client = self.client.clone();

        move |context| {
            async move {
                client
                    .resolve()
                    .map_err(RequestError::Gh)?
                    .mark_thread_as_done(id)
                    .await
                    .map_err(RequestError::Gh)?;
                context.emit_gh(GhEvent::NotificationMarkedAsDone {
                    notification_id: id,
                });
                Ok(())
            }
            .boxed()
        }
    }

    pub(super) fn unsubscribe_thread(
        &self,
        id: ThreadId,
    ) -> impl FnOnce(RequestContext) -> RequestFuture + use<> {
        let client = self.client.clone();

        move |_context| {
            async move {
                client
                    .resolve()
                    .map_err(RequestError::Gh)?
                    .unsubscribe_thread(id)
                    .await
                    .map_err(RequestError::Gh)
            }
            .boxed()
        }
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
