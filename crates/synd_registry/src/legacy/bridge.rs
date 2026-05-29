use std::collections::HashMap;

use chrono::Utc;
use synd_feed::types::{Annotated, FeedUrl};

use crate::{
    db::{FeedRegistryDb, RegistryDbTransaction},
    error::FeedRegistryError,
    event::{RegistryNotification, RegistryNotificationPublisher, TimelineChanged},
};

use super::{
    executor::RefreshExecutorHandle,
    model::{
        EffectiveRefreshPolicy, EntriesPage, EntryCursor, EntryView, FeedRegistryConfig,
        FeedStatusQuery, FeedSubscription, FeedSubscriptionView, FeedSubscriptionsPage,
        InitialRefreshMode, ListEntriesQuery, ListSubscriptionsQuery, ReconcileOutcome,
        ReconcileTrigger, RefreshIntent, RefreshIntentKind, RefreshPolicy, RefreshRequestReceipt,
        RefreshState, RefreshStatus, RefreshStatusKind, RefreshSuccess, RequestRefreshCommand,
        SubscribeFeedCommand, SubscribeFeedOutput, SubscribeFeedRefresh, UnsubscribeFeedCommand,
        UnsubscribeFeedOutput,
    },
    provider::FeedProvider,
    reconciler::Reconciler,
};

#[derive(Clone)]
pub struct LegacyBridge<S, P> {
    db: S,
    provider: P,
    executor: RefreshExecutorHandle,
    reconciler: Reconciler<S>,
    config: FeedRegistryConfig,
    notifications: RegistryNotificationPublisher,
}

impl<S, P> LegacyBridge<S, P>
where
    S: FeedRegistryDb,
    P: FeedProvider,
{
    pub fn new(
        db: S,
        provider: P,
        executor: RefreshExecutorHandle,
        config: FeedRegistryConfig,
    ) -> Self {
        Self::with_notifications(
            db,
            provider,
            executor,
            config,
            RegistryNotificationPublisher::default(),
        )
    }

    pub fn with_notifications(
        db: S,
        provider: P,
        executor: RefreshExecutorHandle,
        config: FeedRegistryConfig,
        notifications: RegistryNotificationPublisher,
    ) -> Self {
        Self {
            db: db.clone(),
            provider,
            executor: executor.clone(),
            reconciler: Reconciler::new(db, executor),
            config,
            notifications,
        }
    }

    pub fn subscribe_notifications(&self) -> crate::event::RegistryNotificationSubscriber {
        self.notifications.subscribe()
    }

    pub fn default_refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::interval(self.config.default_refresh_interval)
    }

    pub async fn subscribe(
        &self,
        command: SubscribeFeedCommand,
    ) -> Result<SubscribeFeedOutput, FeedRegistryError> {
        let now = Utc::now();
        let subscription = FeedSubscription {
            subscriber_id: command.subscriber_id.clone(),
            feed_url: command.feed_url.clone(),
            requirement: command.requirement,
            category: command.category,
            refresh_policy: command.refresh_policy,
            created_at: now,
            updated_at: now,
        };
        let refresh = match command.initial_refresh {
            InitialRefreshMode::Async => {
                let mut tx = self.db.begin().await?;
                tx.upsert_subscription(subscription.clone()).await?;
                tx.commit().await?;
                let receipt = self
                    .executor
                    .submit(RefreshIntent::new(
                        command.feed_url,
                        RefreshIntentKind::Initial,
                        Some(command.subscriber_id),
                        now,
                    ))
                    .await;
                SubscribeFeedRefresh::Enqueued(receipt)
            }
            InitialRefreshMode::RequireSuccess => {
                let fetched = self
                    .provider
                    .fetch(command.feed_url.clone())
                    .await
                    .map_err(|err| FeedRegistryError::InitialRefreshFailed(err.to_string()))?;
                let succeeded_at = Utc::now();
                let mut tx = self.db.begin().await?;
                tx.upsert_subscription(subscription.clone()).await?;
                let subscriptions = tx
                    .list_active_subscriptions_for_feed(&subscription.feed_url)
                    .await?;
                let policy = EffectiveRefreshPolicy::from_subscriptions(&subscriptions)
                    .expect("subscription is visible in transaction after upsert");
                let next_refresh_after = policy.next_after(succeeded_at);
                tx.record_refresh_succeeded(RefreshSuccess {
                    snapshot: fetched.snapshot,
                    succeeded_at,
                    next_refresh_after,
                })
                .await?;
                tx.commit().await?;
                self.publish_timeline_changed(TimelineChanged::for_feed(
                    subscription.feed_url.clone(),
                    succeeded_at,
                ));

                SubscribeFeedRefresh::Completed(RefreshStatus {
                    feed_url: subscription.feed_url.clone(),
                    kind: RefreshStatusKind::Idle,
                    active_request_id: None,
                    last_attempt_at: Some(succeeded_at),
                    last_success_at: Some(succeeded_at),
                    last_failure_at: None,
                    last_error_message: None,
                })
            }
        };

        Ok(SubscribeFeedOutput {
            subscription,
            refresh,
        })
    }

    pub async fn unsubscribe(
        &self,
        command: UnsubscribeFeedCommand,
    ) -> Result<UnsubscribeFeedOutput, FeedRegistryError> {
        let mut tx = self.db.begin().await?;
        tx.delete_subscription(&command.subscriber_id, &command.feed_url)
            .await?;
        let remaining = tx
            .list_active_subscriptions_for_feed(&command.feed_url)
            .await?;
        if remaining.is_empty() {
            tx.delete_feed_state(&command.feed_url).await?;
        }
        tx.commit().await?;
        if remaining.is_empty() {
            self.executor.cancel(&command.feed_url).await;
        }
        self.publish_timeline_changed(TimelineChanged::for_feed(command.feed_url, Utc::now()));
        Ok(UnsubscribeFeedOutput {})
    }

    pub async fn request_refresh(
        &self,
        command: RequestRefreshCommand,
    ) -> Result<RefreshRequestReceipt, FeedRegistryError> {
        let now = Utc::now();
        let mut tx = self.db.begin().await?;
        if !tx
            .has_subscription(&command.subscriber_id, &command.feed_url)
            .await?
        {
            return Err(FeedRegistryError::NotSubscribed(command.feed_url));
        }
        let intent = RefreshIntent::new(
            command.feed_url,
            RefreshIntentKind::Manual,
            Some(command.subscriber_id),
            now,
        );
        tx.commit().await?;
        Ok(self.executor.submit(intent).await)
    }

    pub async fn reconcile_now(
        &self,
        trigger: ReconcileTrigger,
    ) -> Result<ReconcileOutcome, FeedRegistryError> {
        self.reconciler.reconcile_now(trigger).await
    }

    pub async fn list_subscriptions(
        &self,
        query: ListSubscriptionsQuery,
    ) -> Result<FeedSubscriptionsPage, FeedRegistryError> {
        let mut tx = self.db.begin().await?;
        let page = tx.list_subscriptions(query).await?;
        let urls = page
            .nodes
            .iter()
            .map(|sub| sub.feed_url.clone())
            .collect::<Vec<_>>();
        let snapshots = tx.load_snapshots(&urls).await?;
        let states = tx.load_refresh_states(&urls).await?;
        tx.commit().await?;

        let snapshot_by_url = snapshots
            .into_iter()
            .map(|snapshot| (snapshot.feed_url.clone(), snapshot))
            .collect::<HashMap<_, _>>();
        let state_by_url = states
            .into_iter()
            .map(|state| (state.feed_url.clone(), state))
            .collect::<HashMap<_, _>>();
        let mut nodes = Vec::with_capacity(page.nodes.len());
        for subscription in page.nodes {
            let active_status = self.executor.active_status(&subscription.feed_url).await;
            let refresh_status = active_status.unwrap_or_else(|| {
                build_status(
                    subscription.feed_url.clone(),
                    state_by_url.get(&subscription.feed_url),
                )
            });
            let feed = snapshot_by_url
                .get(&subscription.feed_url)
                .and_then(|snapshot| self.provider.parse_snapshot(snapshot).ok());
            nodes.push(FeedSubscriptionView {
                subscription,
                feed,
                refresh_status,
            });
        }

        Ok(FeedSubscriptionsPage {
            nodes,
            has_next_page: page.has_next_page,
            end_cursor: page.end_cursor,
        })
    }

    pub async fn list_entries(
        &self,
        query: ListEntriesQuery,
    ) -> Result<EntriesPage, FeedRegistryError> {
        let mut tx = self.db.begin().await?;
        let subscriptions = tx
            .list_subscriptions_for_subscriber(&query.subscriber_id)
            .await?;
        let urls = subscriptions
            .iter()
            .map(|sub| sub.feed_url.clone())
            .collect::<Vec<_>>();
        let snapshots = tx.load_snapshots(&urls).await?;
        tx.commit().await?;

        let snapshot_by_url = snapshots
            .into_iter()
            .map(|snapshot| (snapshot.feed_url.clone(), snapshot))
            .collect::<HashMap<_, _>>();

        let mut entries = Vec::new();
        for subscription in subscriptions {
            let Some(snapshot) = snapshot_by_url.get(&subscription.feed_url) else {
                continue;
            };
            let Ok(feed) = self.provider.parse_snapshot(snapshot) else {
                continue;
            };
            let meta = Annotated {
                feed: feed.meta().clone(),
                requirement: subscription.requirement,
                category: subscription.category.clone(),
            };
            entries.extend(
                feed.entries()
                    .cloned()
                    .enumerate()
                    .map(|(ordinal, entry)| EntryView {
                        cursor: EntryCursor::for_entry(
                            subscription.feed_url.clone(),
                            &entry,
                            ordinal,
                        ),
                        entry,
                        feed_meta: meta.clone(),
                    }),
            );
        }

        entries.sort_unstable_by(|a, b| a.cursor.sort_cmp(&b.cursor));

        let start = query.after.as_ref().map_or(0, |after| {
            entries
                .iter()
                .position(|entry| entry.cursor.is_after(after))
                .unwrap_or(entries.len())
        });

        let mut page = entries.into_iter().skip(start).collect::<Vec<_>>();
        let has_next_page = page.len() > query.first;
        page.truncate(query.first);
        let end_cursor = page.last().map(|entry| entry.cursor.clone());

        Ok(EntriesPage {
            nodes: page,
            has_next_page,
            end_cursor,
        })
    }

    pub async fn feed_status(
        &self,
        query: FeedStatusQuery,
    ) -> Result<RefreshStatus, FeedRegistryError> {
        let mut tx = self.db.begin().await?;
        if !tx
            .has_subscription(&query.subscriber_id, &query.feed_url)
            .await?
        {
            return Err(FeedRegistryError::NotSubscribed(query.feed_url));
        }
        let states = tx
            .load_refresh_states(std::slice::from_ref(&query.feed_url))
            .await?;
        tx.commit().await?;
        Ok(self
            .executor
            .active_status(&query.feed_url)
            .await
            .unwrap_or_else(|| build_status(query.feed_url, states.first())))
    }

    fn publish_timeline_changed(&self, event: TimelineChanged) {
        self.notifications
            .publish(RegistryNotification::TimelineChanged(event));
    }
}

fn build_status(feed_url: FeedUrl, state: Option<&RefreshState>) -> RefreshStatus {
    let kind = match state {
        None => RefreshStatusKind::NeverRefreshed,
        Some(state) if state.last_success_at.is_none() && state.last_failure_at.is_none() => {
            RefreshStatusKind::NeverRefreshed
        }
        Some(state) if state.last_failure_at > state.last_success_at => {
            RefreshStatusKind::LastFailed
        }
        Some(_) => RefreshStatusKind::Idle,
    };

    RefreshStatus {
        feed_url,
        kind,
        active_request_id: None,
        last_attempt_at: state.and_then(|state| state.last_attempt_at),
        last_success_at: state.and_then(|state| state.last_success_at),
        last_failure_at: state.and_then(|state| state.last_failure_at),
        last_error_message: state.and_then(|state| state.last_error_message.clone()),
    }
}
