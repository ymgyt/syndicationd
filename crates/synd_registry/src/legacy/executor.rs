use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Duration,
};

use chrono::Utc;
use futures_util::{StreamExt, stream::FuturesUnordered};
use synd_feed::types::FeedUrl;
use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::{
    db::{FeedRegistryDb, RegistryDbTransaction},
    event::{RegistryNotification, RegistryNotificationPublisher, TimelineChanged},
};

use super::{
    model::{
        ClaimedRefreshRequest, EffectiveRefreshPolicy, FeedRegistryConfig, NewRefreshRequest,
        RefreshErrorKind, RefreshFailure, RefreshIntent, RefreshRequest, RefreshRequestReceipt,
        RefreshRequestStatus, RefreshStarted, RefreshStatus, RefreshStatusKind, RefreshSuccess,
    },
    planner::{RefreshRequestDecision, RefreshRequestPolicy},
    provider::{FeedProvider, FeedProviderError},
};

#[derive(Default)]
struct RefreshQueueState {
    by_url: HashMap<FeedUrl, RefreshRequest>,
    pending: VecDeque<FeedUrl>,
}

#[derive(Clone)]
pub struct RefreshExecutorHandle {
    notify: Arc<Notify>,
    queue: Arc<Mutex<RefreshQueueState>>,
}

impl RefreshExecutorHandle {
    pub fn new() -> Self {
        Self {
            notify: Arc::new(Notify::new()),
            queue: Arc::new(Mutex::new(RefreshQueueState::default())),
        }
    }

    pub async fn submit(&self, intent: RefreshIntent) -> RefreshRequestReceipt {
        let mut queue = self.queue.lock().await;
        let active = queue.by_url.get(&intent.feed_url).cloned();
        let decision = RefreshRequestPolicy::coalesce(intent, active);
        let disposition = decision.disposition();
        let request = match decision {
            RefreshRequestDecision::Create(request) => {
                let request: RefreshRequest = request.into();
                queue.pending.push_back(request.feed_url.clone());
                queue
                    .by_url
                    .insert(request.feed_url.clone(), request.clone());
                request
            }
            RefreshRequestDecision::Promote(update)
            | RefreshRequestDecision::MergePending(update)
            | RefreshRequestDecision::JoinRunning(update) => {
                let request = queue
                    .by_url
                    .get_mut(&update.feed_url)
                    .expect("active request disappeared during coalesce");
                request.intent = update.intent;
                request.priority = update.priority;
                request.requested_by = update.requested_by;
                request.requested_at = update.requested_at;
                request.signal_count = update.signal_count;
                request.not_before = update.not_before;
                request.updated_at = update.updated_at;
                request.clone()
            }
        };
        drop(queue);
        self.notify.notify_one();

        RefreshRequestReceipt {
            request_id: request.id.clone(),
            disposition,
            status: status_from_request(&request),
        }
    }

    pub async fn active_status(&self, feed_url: &FeedUrl) -> Option<RefreshStatus> {
        let queue = self.queue.lock().await;
        queue.by_url.get(feed_url).map(status_from_request)
    }

    pub async fn cancel(&self, feed_url: &FeedUrl) {
        let mut queue = self.queue.lock().await;
        queue.by_url.remove(feed_url);
        queue.pending.retain(|pending| pending != feed_url);
    }

    async fn claim_next(
        &self,
        now: chrono::DateTime<Utc>,
        lease_duration: Duration,
    ) -> Option<ClaimedRefreshRequest> {
        let mut queue = self.queue.lock().await;
        let next = queue
            .pending
            .iter()
            .enumerate()
            .filter_map(|(position, feed_url)| {
                let request = queue.by_url.get(feed_url)?;
                (request.status == RefreshRequestStatus::Pending && request.not_before <= now)
                    .then_some((position, request.priority))
            })
            .fold(None, |best, candidate| match best {
                None => Some(candidate),
                Some((_, priority)) if candidate.1 > priority => Some(candidate),
                Some(best) => Some(best),
            })
            .map(|(position, _)| position)?;

        let feed_url = queue.pending.remove(next)?;
        let request = queue
            .by_url
            .get_mut(&feed_url)
            .expect("pending refresh request disappeared during claim");

        let lease_until = add_duration(now, lease_duration);
        request.status = RefreshRequestStatus::Running;
        request.attempt_count = request.attempt_count.saturating_add(1);
        request.lease_until = Some(lease_until);
        request.updated_at = now;

        Some(ClaimedRefreshRequest {
            id: request.id.clone(),
            feed_url: request.feed_url.clone(),
            lease_until,
            attempt_count: request.attempt_count,
        })
    }

    async fn is_current(&self, request: &ClaimedRefreshRequest) -> bool {
        let queue = self.queue.lock().await;
        queue.by_url.get(&request.feed_url).is_some_and(|active| {
            active.id == request.id && active.status == RefreshRequestStatus::Running
        })
    }

    async fn release(&self, request: &ClaimedRefreshRequest, not_before: chrono::DateTime<Utc>) {
        let mut queue = self.queue.lock().await;
        let Some(active) = queue.by_url.get_mut(&request.feed_url) else {
            return;
        };
        if active.id != request.id || active.status != RefreshRequestStatus::Running {
            return;
        }

        active.status = RefreshRequestStatus::Pending;
        active.not_before = not_before;
        active.lease_until = None;
        active.updated_at = Utc::now();
        if !queue.pending.iter().any(|url| url == &request.feed_url) {
            queue.pending.push_back(request.feed_url.clone());
        }
        drop(queue);
        self.notify.notify_one();
    }

    async fn complete(&self, request: &ClaimedRefreshRequest) {
        let mut queue = self.queue.lock().await;
        let Some(active) = queue.by_url.get(&request.feed_url) else {
            return;
        };
        if active.id != request.id {
            return;
        }

        queue.by_url.remove(&request.feed_url);
        queue.pending.retain(|pending| pending != &request.feed_url);
    }
}

impl Default for RefreshExecutorHandle {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RefreshExecutor<S, P> {
    db: S,
    provider: P,
    handle: RefreshExecutorHandle,
    config: FeedRegistryConfig,
    notifications: RegistryNotificationPublisher,
}

enum ActiveFeedPolicy {
    Present(EffectiveRefreshPolicy),
    Absent,
}

impl<S, P> RefreshExecutor<S, P>
where
    S: FeedRegistryDb,
    P: FeedProvider,
{
    pub fn new(
        db: S,
        provider: P,
        handle: RefreshExecutorHandle,
        config: FeedRegistryConfig,
    ) -> Self {
        Self::with_notifications(
            db,
            provider,
            handle,
            config,
            RegistryNotificationPublisher::default(),
        )
    }

    pub fn with_notifications(
        db: S,
        provider: P,
        handle: RefreshExecutorHandle,
        config: FeedRegistryConfig,
        notifications: RegistryNotificationPublisher,
    ) -> Self {
        Self {
            db,
            provider,
            handle,
            config,
            notifications,
        }
    }

    pub async fn run(self, cancellation: CancellationToken) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        let mut in_flight = FuturesUnordered::new();

        loop {
            while in_flight.len() < self.config.refresh_concurrency {
                let Some(request) = self
                    .handle
                    .claim_next(Utc::now(), self.config.refresh_lease_duration)
                    .await
                else {
                    break;
                };
                in_flight.push(self.execute(request));
            }

            tokio::select! {
                () = cancellation.cancelled() => break,
                Some(()) = in_flight.next(), if !in_flight.is_empty() => {},
                () = self.handle.notify.notified() => {},
                _ = interval.tick() => {},
            }
        }
    }

    async fn execute(&self, request: ClaimedRefreshRequest) {
        debug!(request_id = %request.id, feed_url = %request.feed_url, "execute refresh request");

        match self.active_policy(&request).await {
            Ok(ActiveFeedPolicy::Present(_)) => {}
            Ok(ActiveFeedPolicy::Absent) => {
                self.handle.complete(&request).await;
                return;
            }
            Err(err) => {
                warn!("failed to load active refresh policy: {err}");
                self.release_after_db_error(&request).await;
                return;
            }
        }

        let started_at = Utc::now();
        if let Err(err) = self.record_started(&request, started_at).await {
            warn!("failed to record refresh start: {err}");
            self.release_after_db_error(&request).await;
            return;
        }

        match self.provider.fetch(request.feed_url.clone()).await {
            Ok(fetched) => {
                if !self.handle.is_current(&request).await {
                    return;
                }
                let succeeded_at = Utc::now();
                let policy = match self.active_policy(&request).await {
                    Ok(ActiveFeedPolicy::Present(policy)) => policy,
                    Ok(ActiveFeedPolicy::Absent) => {
                        self.handle.complete(&request).await;
                        return;
                    }
                    Err(err) => {
                        warn!("failed to load active refresh policy: {err}");
                        self.release_after_db_error(&request).await;
                        return;
                    }
                };
                let result = RefreshSuccess {
                    snapshot: fetched.snapshot,
                    succeeded_at,
                    next_refresh_after: policy.next_after(succeeded_at),
                };
                if let Err(err) = self.record_success(result).await {
                    warn!("failed to record refresh success: {err}");
                    self.release_after_db_error(&request).await;
                    return;
                }
                self.publish_timeline_changed(TimelineChanged::for_feed(
                    request.feed_url.clone(),
                    succeeded_at,
                ));
                self.handle.complete(&request).await;
            }
            Err(err) => {
                if !self.handle.is_current(&request).await {
                    return;
                }
                let failed_at = Utc::now();
                let policy = match self.active_policy(&request).await {
                    Ok(ActiveFeedPolicy::Present(policy)) => policy,
                    Ok(ActiveFeedPolicy::Absent) => {
                        self.handle.complete(&request).await;
                        return;
                    }
                    Err(err) => {
                        warn!("failed to load active refresh policy: {err}");
                        self.release_after_db_error(&request).await;
                        return;
                    }
                };
                let failure = RefreshFailure {
                    feed_url: request.feed_url.clone(),
                    failed_at,
                    error_kind: error_kind(&err),
                    error_message: err.to_string(),
                    next_refresh_after: policy.next_after(failed_at),
                };
                if let Err(err) = self.record_failure(failure).await {
                    warn!("failed to record refresh failure: {err}");
                    self.release_after_db_error(&request).await;
                    return;
                }
                self.handle.complete(&request).await;
            }
        }
    }

    async fn active_policy(
        &self,
        request: &ClaimedRefreshRequest,
    ) -> Result<ActiveFeedPolicy, crate::error::RegistryDbError> {
        if !self.handle.is_current(request).await {
            return Ok(ActiveFeedPolicy::Absent);
        }

        self.load_effective_policy(&request.feed_url).await
    }

    async fn load_effective_policy(
        &self,
        feed_url: &FeedUrl,
    ) -> Result<ActiveFeedPolicy, crate::error::RegistryDbError> {
        let mut tx = self.db.begin().await?;
        let subscriptions = tx.list_active_subscriptions_for_feed(feed_url).await?;
        tx.commit().await?;
        Ok(EffectiveRefreshPolicy::from_subscriptions(&subscriptions)
            .map_or(ActiveFeedPolicy::Absent, ActiveFeedPolicy::Present))
    }

    async fn release_after_db_error(&self, request: &ClaimedRefreshRequest) {
        self.handle
            .release(
                request,
                add_duration(Utc::now(), self.config.db_retry_delay),
            )
            .await;
    }

    async fn record_started(
        &self,
        request: &ClaimedRefreshRequest,
        started_at: chrono::DateTime<Utc>,
    ) -> Result<(), crate::error::RegistryDbError> {
        let mut tx = self.db.begin().await?;
        tx.record_refresh_started(RefreshStarted {
            feed_url: request.feed_url.clone(),
            started_at,
        })
        .await?;
        tx.commit().await
    }

    async fn record_success(
        &self,
        result: RefreshSuccess,
    ) -> Result<(), crate::error::RegistryDbError> {
        let mut tx = self.db.begin().await?;
        tx.record_refresh_succeeded(result).await?;
        tx.commit().await
    }

    async fn record_failure(
        &self,
        result: RefreshFailure,
    ) -> Result<(), crate::error::RegistryDbError> {
        let mut tx = self.db.begin().await?;
        tx.record_refresh_failed(result).await?;
        tx.commit().await
    }

    fn publish_timeline_changed(&self, event: TimelineChanged) {
        self.notifications
            .publish(RegistryNotification::TimelineChanged(event));
    }
}

impl From<NewRefreshRequest> for RefreshRequest {
    fn from(request: NewRefreshRequest) -> Self {
        Self {
            id: request.id,
            feed_url: request.feed_url,
            intent: request.intent,
            priority: request.priority,
            requested_by: request.requested_by,
            requested_at: request.requested_at,
            signal_count: request.signal_count,
            not_before: request.not_before,
            status: request.status,
            attempt_count: request.attempt_count,
            lease_until: request.lease_until,
            created_at: request.created_at,
            updated_at: request.updated_at,
        }
    }
}

fn status_from_request(request: &RefreshRequest) -> RefreshStatus {
    RefreshStatus {
        feed_url: request.feed_url.clone(),
        kind: match request.status {
            RefreshRequestStatus::Pending => RefreshStatusKind::Pending,
            RefreshRequestStatus::Running => RefreshStatusKind::Running,
        },
        active_request_id: Some(request.id.clone()),
        last_attempt_at: None,
        last_success_at: None,
        last_failure_at: None,
        last_error_message: None,
    }
}

fn add_duration(
    time: chrono::DateTime<chrono::Utc>,
    duration: Duration,
) -> chrono::DateTime<chrono::Utc> {
    chrono::Duration::from_std(duration).map_or(time, |duration| time + duration)
}

fn error_kind(err: &FeedProviderError) -> RefreshErrorKind {
    use synd_feed::feed::service::FetchFeedError;

    match err {
        FeedProviderError::Fetch(FetchFeedError::Fetch(_)) => RefreshErrorKind::Fetch,
        FeedProviderError::Fetch(FetchFeedError::ResponseLimitExceed) => {
            RefreshErrorKind::ResponseLimitExceeded
        }
        FeedProviderError::Fetch(FetchFeedError::InvalidFeed(_)) => RefreshErrorKind::InvalidFeed,
        FeedProviderError::Fetch(FetchFeedError::Io(_)) => RefreshErrorKind::Io,
        FeedProviderError::Fetch(FetchFeedError::JsonFormat(_)) => RefreshErrorKind::JsonFormat,
        FeedProviderError::Fetch(FetchFeedError::JsonUnsupportedVersion(_)) => {
            RefreshErrorKind::JsonUnsupportedVersion
        }
        FeedProviderError::Fetch(FetchFeedError::XmlFormat(_)) => RefreshErrorKind::XmlFormat,
        FeedProviderError::Fetch(FetchFeedError::Other(_)) => RefreshErrorKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::legacy::model::{RefreshIntentKind, SubscriberId};

    fn feed_url(path: &str) -> FeedUrl {
        FeedUrl::parse(&format!("https://example.com/{path}.xml")).unwrap()
    }

    #[tokio::test]
    async fn claim_next_prefers_highest_priority_request() {
        let handle = RefreshExecutorHandle::new();
        let now = Utc::now();
        let scheduled = feed_url("scheduled");
        let manual = feed_url("manual");

        handle
            .submit(RefreshIntent::new(
                scheduled.clone(),
                RefreshIntentKind::Scheduled,
                None,
                now,
            ))
            .await;
        handle
            .submit(RefreshIntent::new(
                manual.clone(),
                RefreshIntentKind::Manual,
                Some(SubscriberId::new("local")),
                now,
            ))
            .await;

        let claimed = handle
            .claim_next(now, Duration::from_mins(1))
            .await
            .expect("refresh request should be claimable");

        assert_eq!(claimed.feed_url, manual);
        assert!(handle.active_status(&scheduled).await.is_some());
        assert!(matches!(
            handle.active_status(&claimed.feed_url).await.unwrap().kind,
            RefreshStatusKind::Running
        ));
    }

    #[tokio::test]
    async fn cancel_removes_pending_request() {
        let handle = RefreshExecutorHandle::new();
        let now = Utc::now();
        let feed_url = feed_url("feed");

        handle
            .submit(RefreshIntent::new(
                feed_url.clone(),
                RefreshIntentKind::Manual,
                None,
                now,
            ))
            .await;
        handle.cancel(&feed_url).await;

        assert!(handle.active_status(&feed_url).await.is_none());
        assert!(
            handle
                .claim_next(now, Duration::from_mins(1))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn stale_running_request_does_not_complete_new_request_for_same_feed() {
        let handle = RefreshExecutorHandle::new();
        let now = Utc::now();
        let feed_url = feed_url("feed");

        handle
            .submit(RefreshIntent::new(
                feed_url.clone(),
                RefreshIntentKind::Manual,
                None,
                now,
            ))
            .await;
        let stale = handle
            .claim_next(now, Duration::from_mins(1))
            .await
            .expect("first request should be claimable");
        handle.cancel(&feed_url).await;
        let receipt = handle
            .submit(RefreshIntent::new(
                feed_url.clone(),
                RefreshIntentKind::Initial,
                Some(SubscriberId::new("local")),
                now,
            ))
            .await;

        handle.complete(&stale).await;

        let status = handle
            .active_status(&feed_url)
            .await
            .expect("new request should remain active");
        assert_eq!(status.active_request_id, Some(receipt.request_id));
        assert!(matches!(status.kind, RefreshStatusKind::Pending));
    }
}
