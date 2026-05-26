use std::{ops::Sub, pin::Pin, sync::Arc, time::Duration};

use futures_util::FutureExt;
use tokio::{sync::mpsc, task::JoinHandle, time::Sleep};
use update_informer::{Check, registry};

use crate::{
    application::{
        Authenticator, Cache, Clock, FEED_REFRESH_POLL_INTERVAL, FEED_VIEW_SYNC_INTERVAL,
        FeedApiSession, RequestId, TIMELINE_INVALIDATION_DEBOUNCE, input_parser::InputParser,
    },
    auth::{Credential, Verified},
    client::{
        github::FetchNotificationsParams,
        github::GithubClient,
        synd_api::{Client, payload},
    },
    config,
    event::{ApiEvent, AuthApiEvent, Event, FeedsApiEvent, GitHubApiEvent},
    interact::Interact,
    job::Jobs,
    local_api::LocalApiRuntime,
    operation::Operation,
    terminal::Terminal,
    types::github::{IssueOrPullRequest, NotificationId, ThreadId},
};

const TIMELINE_CHANGE_RECONNECT_DELAY: Duration = Duration::from_secs(2);

/// Outside-world adapters and execution machinery for component operations.
pub(super) struct Drivers {
    pub(super) clock: Box<dyn Clock>,
    pub(super) terminal: Terminal,
    pub(super) client: Client,
    pub(super) feed_api_session: FeedApiSession,
    pub(super) github_client: Option<GithubClient>,
    pub(super) local_api_runtime: Option<LocalApiRuntime>,
    pub(super) jobs: Jobs,
    pub(super) background_jobs: Jobs,
    pub(super) interactor: Box<dyn Interact>,
    pub(super) authenticator: Authenticator,
    pub(super) in_flight: super::InFlight,
    pub(super) timeline_changes: TimelineChangeSubscription,
    pub(super) cache: Cache,
    pub(super) idle_timer: Pin<Box<Sleep>>,
}

/// Running GraphQL timeline change subscription and its event receiver.
pub(super) struct TimelineChangeSubscription {
    rx: mpsc::UnboundedReceiver<payload::TimelineChangeEvent>,
    task: Option<JoinHandle<()>>,
}

impl TimelineChangeSubscription {
    pub(super) fn new() -> Self {
        let (_tx, rx) = mpsc::unbounded_channel();
        Self { rx, task: None }
    }

    pub(super) fn start(&mut self, client: Client) {
        if self.task.is_some() {
            return;
        }

        let (tx, rx) = mpsc::unbounded_channel();
        self.rx = rx;
        self.task = Some(tokio::spawn(async move {
            loop {
                if tx.is_closed() {
                    break;
                }

                match client.run_timeline_changes(tx.clone()).await {
                    Ok(()) => tracing::debug!("timeline change subscription stopped"),
                    Err(error) => tracing::warn!("timeline change subscription failed: {error}"),
                }

                if tx.is_closed() {
                    break;
                }
                tokio::time::sleep(TIMELINE_CHANGE_RECONNECT_DELAY).await;
            }
        }));
    }

    pub(super) fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        let (_tx, rx) = mpsc::unbounded_channel();
        self.rx = rx;
    }

    pub(super) fn restart_if_running(&mut self, client: Client) -> bool {
        if self.task.is_none() {
            return false;
        }
        self.stop();
        self.start(client);
        true
    }

    pub(super) async fn recv(&mut self) -> Option<payload::TimelineChangeEvent> {
        self.rx.recv().await
    }
}

impl Drivers {
    pub(super) fn set_credential(&mut self, cred: Verified<Credential>) {
        self.schedule_credential_refreshing(&cred);
        self.client.set_credential(cred);
    }

    pub(super) fn persist_credential(
        &mut self,
        cred: &Verified<Credential>,
    ) -> Result<(), super::PersistCacheError> {
        self.cache.persist_credential(cred)
    }

    pub(super) fn perform_operation(&mut self, operation: Operation) -> Vec<Event> {
        match operation {
            Operation::StartDeviceFlow { provider } => self.init_device_flow(provider),
            Operation::PollDeviceFlowAccessToken {
                provider,
                device_authorization,
            } => self.poll_device_flow_access_token(provider, *device_authorization),
            Operation::OpenFeedSubscriptionEditor => self.open_feed_subscription_editor(),
            Operation::OpenFeedEditionEditor { prompt } => {
                self.open_feed_edition_editor(prompt.as_str())
            }
            Operation::SubscribeFeed { input } => {
                self.subscribe_feed(input);
                Vec::new()
            }
            Operation::RefreshFeed { url } => self.refresh_feed(url),
            Operation::FetchFeedRefreshStatus {
                url,
                request_id,
                remaining,
            } => {
                self.fetch_feed_refresh_status(url, request_id, remaining);
                Vec::new()
            }
            Operation::ScheduleFeedRefreshPoll {
                url,
                request_id,
                remaining,
            } => {
                self.schedule_feed_refresh_poll(url, request_id, remaining);
                Vec::new()
            }
            Operation::FetchSubscription {
                populate,
                after,
                first,
            } => {
                self.fetch_subscription(populate, after, first);
                Vec::new()
            }
            Operation::FetchEntries {
                populate,
                after,
                first,
            } => self.fetch_entries(populate, after, first, false),
            Operation::FetchInitialFeedView {
                subscriptions_first,
                timeline_first,
            } => self.fetch_initial_feed_view(subscriptions_first, timeline_first),
            Operation::RefetchTimelineEntries {
                populate,
                after,
                first,
            } => self.fetch_entries(populate, after, first, true),
            Operation::StartTimelineChangeSubscription => {
                let client = self.client.clone();
                self.timeline_changes.start(client);
                Vec::new()
            }
            Operation::UnsubscribeFeed { url } => {
                self.unsubscribe_feed(url);
                Vec::new()
            }
            Operation::ScheduleFeedViewSync => {
                self.schedule_feed_view_sync();
                Vec::new()
            }
            Operation::ScheduleTimelineReload => {
                self.schedule_debounced_timeline_reload();
                Vec::new()
            }
            Operation::FetchGitHubNotifications { populate, params } => {
                self.fetch_gh_notifications(populate, params)
            }
            Operation::FetchGitHubNotificationDetails { contexts } => {
                self.fetch_gh_notification_details(contexts)
            }
            Operation::MarkGitHubNotificationAsDone { id } => {
                self.mark_gh_notification_as_done_by_id(id)
            }
            Operation::UnsubscribeGitHubThread { id } => self.unsubscribe_gh_thread_by_id(id),
            Operation::OpenBrowser { url } => match self.interactor.open_browser(url) {
                Ok(()) => Vec::new(),
                Err(err) => vec![Event::Error {
                    message: format!("open browser: {err}"),
                }],
            },
            Operation::OpenTextBrowser { url } => match self.interactor.open_text_browser(url) {
                Ok(()) => Vec::new(),
                Err(err) => vec![Event::Error {
                    message: format!("open browser: {err}"),
                }],
            },
            Operation::ForceRedrawTerminal => {
                self.terminal.force_redraw();
                Vec::new()
            }
            Operation::CheckLatestRelease => {
                self.check_latest_release();
                Vec::new()
            }
        }
    }

    fn schedule_credential_refreshing(&mut self, cred: &Verified<Credential>) {
        match &**cred {
            Credential::Github { .. } => {}
            Credential::Google {
                refresh_token,
                expired_at,
                ..
            } => {
                let until_expire = expired_at
                    .sub(config::credential::EXPIRE_MARGIN)
                    .sub(self.clock.now())
                    .to_std()
                    .unwrap_or(config::credential::FALLBACK_EXPIRE);
                let jwt_service = self.authenticator.jwt_service.clone();
                let refresh_token = refresh_token.clone();
                let fut = async move {
                    tokio::time::sleep(until_expire).await;

                    tracing::debug!("Refresh google credential");
                    match jwt_service.refresh_google_id_token(&refresh_token).await {
                        Ok(credential) => Ok(Event::CredentialRefreshed { credential }),
                        Err(err) => Ok(Event::Error {
                            message: err.to_string(),
                        }),
                    }
                }
                .boxed();
                self.background_jobs.push(fut);
            }
        }
    }

    fn init_device_flow(&mut self, provider: crate::auth::AuthenticationProvider) -> Vec<Event> {
        tracing::info!("Start authenticate");

        let authenticator = self.authenticator.clone();
        let request_seq = self.in_flight.add(RequestId::DeviceFlowDeviceAuthorize);
        let fut = async move {
            match authenticator.init_device_flow(provider).await {
                Ok(device_authorization) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Auth(AuthApiEvent::DeviceFlowAuthorizationReceived {
                        provider,
                        device_authorization: Box::new(device_authorization),
                    }),
                }),
                Err(err) => Ok(Event::oauth_api_error(err, request_seq)),
            }
        }
        .boxed();
        self.jobs.push(fut);
        Vec::new()
    }

    fn poll_device_flow_access_token(
        &mut self,
        provider: crate::auth::AuthenticationProvider,
        device_authorization: synd_auth::device_flow::DeviceAuthorizationResponse,
    ) -> Vec<Event> {
        let authenticator = self.authenticator.clone();
        let now = self.clock.now();
        let request_seq = self.in_flight.add(RequestId::DeviceFlowPollAccessToken);
        let fut = async move {
            match authenticator
                .poll_device_flow_access_token(now, provider, device_authorization)
                .await
            {
                Ok(credential) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Auth(AuthApiEvent::DeviceFlowCredentialReceived {
                        credential,
                    }),
                }),
                Err(err) => Ok(Event::oauth_api_error(err, request_seq)),
            }
        }
        .boxed();

        self.jobs.push(fut);
        Vec::new()
    }

    fn open_feed_subscription_editor(&mut self) -> Vec<Event> {
        match self
            .interactor
            .open_editor(InputParser::SUSBSCRIBE_FEED_PROMPT)
        {
            Ok(input) => {
                tracing::debug!("Got user modified feed subscription: {input}");
                self.terminal.force_redraw();
                vec![Event::FeedSubscriptionEditorClosed { input }]
            }
            Err(err) => {
                tracing::warn!("{err}");
                vec![Event::Error {
                    message: err.to_string(),
                }]
            }
        }
    }

    fn open_feed_edition_editor(&mut self, prompt: &str) -> Vec<Event> {
        match self.interactor.open_editor(prompt) {
            Ok(input) => {
                self.terminal.force_redraw();
                vec![Event::FeedEditionEditorClosed { input }]
            }
            Err(err) => {
                tracing::warn!("{err}");
                vec![Event::Error {
                    message: err.to_string(),
                }]
            }
        }
    }

    fn subscribe_feed(&mut self, input: payload::SubscribeFeedInput) {
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::SubscribeFeed);
        let fut = async move {
            match client.subscribe_feed(input).await {
                Ok(payload) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::FeedSubscribed {
                        url: payload.url.clone(),
                        payload,
                    }),
                }),
                Err(error) => Ok(Event::synd_api_error(error, request_seq)),
            }
        }
        .boxed();
        self.jobs.push(fut);
    }

    fn refresh_feed(&mut self, url: synd_feed::types::FeedUrl) -> Vec<Event> {
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::RefreshFeed);
        let event = Event::FeedRefreshRequested {
            request_seq,
            url: url.clone(),
        };
        let fut = async move {
            match client.refresh_feed(url.clone()).await {
                Ok(payload) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::FeedRefreshAccepted { url, payload }),
                }),
                Err(error) => Ok(Event::synd_api_error(error, request_seq)),
            }
        }
        .boxed();
        self.jobs.push(fut);
        vec![event]
    }

    fn schedule_feed_refresh_poll(
        &mut self,
        url: synd_feed::types::FeedUrl,
        request_id: String,
        remaining: u16,
    ) {
        let fut = async move {
            tokio::time::sleep(FEED_REFRESH_POLL_INTERVAL).await;
            Ok(Event::FeedRefreshPollElapsed {
                url,
                request_id,
                remaining,
            })
        }
        .boxed();
        self.background_jobs.push(fut);
    }

    fn fetch_feed_refresh_status(
        &mut self,
        url: synd_feed::types::FeedUrl,
        request_id: String,
        remaining: u16,
    ) {
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::FetchFeedStatus);
        let fut = async move {
            match client.fetch_feed_status(url.clone()).await {
                Ok(status) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::FeedRefreshStatusFetched {
                        url,
                        request_id: request_id.clone(),
                        remaining,
                        status,
                    }),
                }),
                Err(err) => Ok(Event::FeedRefreshPollError {
                    url,
                    request_id,
                    error: Arc::new(err),
                    request_seq,
                }),
            }
        }
        .boxed();
        self.jobs.push(fut);
    }

    fn unsubscribe_feed(&mut self, url: synd_feed::types::FeedUrl) {
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::UnsubscribeFeed);
        let fut = async move {
            match client.unsubscribe_feed(url.clone()).await {
                Ok(()) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::FeedUnsubscribed { url }),
                }),
                Err(err) => Ok(Event::synd_api_error(err, request_seq)),
            }
        }
        .boxed();
        self.jobs.push(fut);
    }

    fn fetch_initial_feed_view(
        &mut self,
        subscriptions_first: i64,
        timeline_first: i64,
    ) -> Vec<Event> {
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::FetchSubscription);
        let fut = async move {
            match client
                .fetch_initial_feed_view(subscriptions_first, timeline_first)
                .await
            {
                Ok(payload) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::InitialFeedViewFetched { payload }),
                }),
                Err(err) => Ok(Event::synd_api_error(err, request_seq)),
            }
        }
        .boxed();
        self.jobs.push(fut);
        vec![Event::EntryFetchStarted {
            request_seq,
            populate: crate::application::Populate::Replace,
        }]
    }

    fn fetch_subscription(
        &mut self,
        populate: crate::application::Populate,
        after: Option<String>,
        first: i64,
    ) {
        if first <= 0 {
            return;
        }
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::FetchSubscription);
        let fut = async move {
            match client.fetch_subscription(after, Some(first)).await {
                Ok(subscription) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::SubscriptionFetched {
                        populate,
                        subscription,
                    }),
                }),
                Err(err) => Ok(Event::synd_api_error(err, request_seq)),
            }
        }
        .boxed();
        self.jobs.push(fut);
    }

    fn fetch_entries(
        &mut self,
        populate: crate::application::Populate,
        after: Option<String>,
        first: i64,
        timeline_refetch: bool,
    ) -> Vec<Event> {
        if first <= 0 {
            return Vec::new();
        }
        tracing::debug!(
            ?populate,
            has_after = after.is_some(),
            first,
            timeline_refetch,
            "fetch entries"
        );
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::FetchEntries);
        let mut events = vec![Event::EntryFetchStarted {
            request_seq,
            populate,
        }];
        if timeline_refetch {
            events.push(Event::TimelineRefetchStarted { request_seq });
        }
        let fut = async move {
            match client.fetch_entries(after, first).await {
                Ok(payload) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::EntriesFetched { populate, payload }),
                }),
                Err(error) => Ok(Event::SyndApiError {
                    error: Arc::new(error),
                    request_seq,
                }),
            }
        }
        .boxed();
        self.jobs.push(fut);
        events
    }

    fn schedule_feed_view_sync(&mut self) {
        let fut = async move {
            tokio::time::sleep(FEED_VIEW_SYNC_INTERVAL).await;
            Ok(Event::FeedViewSyncElapsed)
        }
        .boxed();
        self.background_jobs.push(fut);
    }

    fn schedule_debounced_timeline_reload(&mut self) {
        let fut = async move {
            tokio::time::sleep(TIMELINE_INVALIDATION_DEBOUNCE).await;
            Ok(Event::TimelineReloadDebounced)
        }
        .boxed();
        self.background_jobs.push(fut);
    }

    fn fetch_gh_notifications(
        &mut self,
        populate: crate::application::Populate,
        params: FetchNotificationsParams,
    ) -> Vec<Event> {
        let Some(client) = self.github_client.clone() else {
            return vec![Event::Error {
                message: "github client is not configured".to_owned(),
            }];
        };
        let request_seq = self
            .in_flight
            .add(RequestId::FetchGithubNotifications { page: params.page });
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
        self.jobs.push(fut);
        Vec::new()
    }

    fn fetch_gh_notification_details(&mut self, contexts: Vec<IssueOrPullRequest>) -> Vec<Event> {
        let Some(client) = self.github_client.clone() else {
            return vec![Event::Error {
                message: "github client is not configured".to_owned(),
            }];
        };

        for context in contexts {
            let client = client.clone();

            let fut = match context {
                either::Either::Left(issue) => {
                    let request_seq = self
                        .in_flight
                        .add(RequestId::FetchGithubIssue { id: issue.id });
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
                    let request_seq = self.in_flight.add(RequestId::FetchGithubPullRequest {
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
            self.jobs.push(fut);
        }

        Vec::new()
    }

    fn mark_gh_notification_as_done_by_id(&mut self, id: NotificationId) -> Vec<Event> {
        let Some(client) = self.github_client.clone() else {
            return vec![Event::Error {
                message: "github client is not configured".to_owned(),
            }];
        };
        let request_seq = self
            .in_flight
            .add(RequestId::MarkGithubNotificationAsDone { id });
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
        self.jobs.push(fut);
        Vec::new()
    }

    fn unsubscribe_gh_thread_by_id(&mut self, id: ThreadId) -> Vec<Event> {
        let Some(client) = self.github_client.clone() else {
            return vec![Event::Error {
                message: "github client is not configured".to_owned(),
            }];
        };
        let request_seq = self.in_flight.add(RequestId::UnsubscribeGithubThread);
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
        self.jobs.push(fut);
        Vec::new()
    }

    fn check_latest_release(&mut self) {
        let check = tokio::task::spawn_blocking(|| {
            let name = env!("CARGO_PKG_NAME");
            let version = env!("CARGO_PKG_VERSION");
            #[cfg(not(test))]
            let informer = update_informer::new(registry::Crates, name, version)
                .interval(std::time::Duration::from_hours(24))
                .timeout(std::time::Duration::from_secs(5));

            #[cfg(test)]
            let informer = update_informer::fake(registry::Crates, name, version, "v1.0.0");

            informer.check_version().ok().flatten()
        });
        let fut = async move {
            match check.await {
                Ok(Some(version)) => Ok(Event::LatestReleaseFound(version)),
                _ => Ok(Event::Nop),
            }
        }
        .boxed();
        self.jobs.push(fut);
    }
}
