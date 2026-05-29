use crate::{
    command::RegistryCommand,
    db::FeedRegistryDb,
    error::FeedRegistryError,
    event::{EventSubmitter, RegistryNotificationPublisher, RegistryNotificationSubscriber},
    legacy::{
        FeedProvider, LegacyBridge, RefreshExecutorHandle,
        model::{
            EntriesPage, FeedRegistryConfig, FeedStatusQuery, FeedSubscriptionsPage,
            ListEntriesQuery, ListSubscriptionsQuery, ReconcileOutcome, ReconcileTrigger,
            RefreshPolicy, RefreshRequestReceipt, RefreshStatus, RequestRefreshCommand,
            SubscribeFeedCommand, SubscribeFeedOutput, UnsubscribeFeedCommand,
            UnsubscribeFeedOutput,
        },
    },
};

#[derive(Clone)]
pub struct FeedRegistry<S, P, E> {
    /// Current synchronous registry behavior.
    ///
    /// New event-flow components should be composed beside this field instead
    /// of being folded into the legacy implementation.
    legacy: LegacyBridge<S, P>,
    events: E,
}

impl<S, P, E> FeedRegistry<S, P, E>
where
    S: FeedRegistryDb,
    P: FeedProvider,
    E: EventSubmitter,
{
    pub fn new(
        db: S,
        provider: P,
        executor: RefreshExecutorHandle,
        config: FeedRegistryConfig,
        events: E,
    ) -> Self {
        Self::with_event_runtime(
            db,
            provider,
            executor,
            config,
            RegistryNotificationPublisher::default(),
            events,
        )
    }

    pub fn with_event_runtime(
        db: S,
        provider: P,
        executor: RefreshExecutorHandle,
        config: FeedRegistryConfig,
        notifications: RegistryNotificationPublisher,
        events: E,
    ) -> Self {
        Self {
            legacy: LegacyBridge::with_notifications(db, provider, executor, config, notifications),
            events,
        }
    }

    pub fn subscribe_notifications(&self) -> RegistryNotificationSubscriber {
        self.legacy.subscribe_notifications()
    }

    pub fn default_refresh_policy(&self) -> RefreshPolicy {
        self.legacy.default_refresh_policy()
    }

    pub async fn subscribe(
        &self,
        command: SubscribeFeedCommand,
    ) -> Result<SubscribeFeedOutput, FeedRegistryError> {
        let event = RegistryCommand::from(&command);
        let output = self.legacy.subscribe(command).await?;
        self.events.submit(event.into_events()).await?;
        Ok(output)
    }

    pub async fn unsubscribe(
        &self,
        command: UnsubscribeFeedCommand,
    ) -> Result<UnsubscribeFeedOutput, FeedRegistryError> {
        let event = RegistryCommand::from(&command);
        let output = self.legacy.unsubscribe(command).await?;
        self.events.submit(event.into_events()).await?;
        Ok(output)
    }

    pub async fn request_refresh(
        &self,
        command: RequestRefreshCommand,
    ) -> Result<RefreshRequestReceipt, FeedRegistryError> {
        self.legacy.request_refresh(command).await
    }

    pub async fn reconcile_now(
        &self,
        trigger: ReconcileTrigger,
    ) -> Result<ReconcileOutcome, FeedRegistryError> {
        self.legacy.reconcile_now(trigger).await
    }

    pub async fn list_subscriptions(
        &self,
        query: ListSubscriptionsQuery,
    ) -> Result<FeedSubscriptionsPage, FeedRegistryError> {
        self.legacy.list_subscriptions(query).await
    }

    pub async fn list_entries(
        &self,
        query: ListEntriesQuery,
    ) -> Result<EntriesPage, FeedRegistryError> {
        self.legacy.list_entries(query).await
    }

    pub async fn feed_status(
        &self,
        query: FeedStatusQuery,
    ) -> Result<RefreshStatus, FeedRegistryError> {
        self.legacy.feed_status(query).await
    }
}
