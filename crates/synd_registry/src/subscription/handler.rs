use std::sync::Arc;

use chrono::{DateTime, Utc};
use synd_support::time::Clock;
use tracing::info;

use crate::{
    command::{
        SubscribeFeedCommand, SubscribeFeedOutput, UnsubscribeFeedCommand, UnsubscribeFeedOutput,
    },
    crawl::policy::CrawlPolicy,
    db::{CommitTx, FeedRegistryDb, SubscriptionDb},
    error::{FeedRegistryError, RegistryDbResult},
    event::{EventRecorder, RecordedEvents, SubEvent},
    handler::{CommandHandler, HandledCommand, StateApplier},
    subscription::{SubState, SubscriptionKey},
};

/// Handles subscription commands as DB state changes plus journaled domain events.
#[derive(Clone)]
pub(crate) struct SubHandler<S> {
    db: S,
    default_crawl_policy: CrawlPolicy,
    clock: Arc<dyn Clock>,
    applier: SubStateApplier,
}

impl<S> SubHandler<S> {
    pub(crate) fn new(db: S, default_crawl_policy: CrawlPolicy, clock: Arc<dyn Clock>) -> Self {
        Self {
            db,
            default_crawl_policy,
            clock,
            applier: SubStateApplier,
        }
    }
}

impl SubState {
    /// Loads the current command-time state of one subscriber/feed relation.
    async fn load<Tx>(tx: &mut Tx, subscription: &SubscriptionKey) -> RegistryDbResult<Self>
    where
        Tx: SubscriptionDb + Send,
    {
        if tx
            .has_subscription(&subscription.subscriber_id, &subscription.feed_url)
            .await?
        {
            Ok(Self::Subscribed)
        } else {
            Ok(Self::NotSubscribed)
        }
    }
}

/// Applies subscription domain events to the subscription store transaction.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SubStateApplier;

impl<Tx> StateApplier<Tx> for SubStateApplier
where
    Tx: SubscriptionDb + Send,
{
    type Event = SubEvent;

    async fn apply(
        &self,
        tx: &mut Tx,
        event: &Self::Event,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        match event {
            SubEvent::Subscribed(event) => {
                tx.upsert_subscription(&event.subscription, event.attrs.clone(), now)
                    .await
            }
            SubEvent::Changed(event) => {
                tx.upsert_subscription(&event.subscription, event.attrs.clone(), now)
                    .await
            }
            SubEvent::Unsubscribed(event) => {
                tx.delete_subscription(
                    &event.subscription.subscriber_id,
                    &event.subscription.feed_url,
                )
                .await
            }
        }
    }
}

impl<S> SubHandler<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: SubscriptionDb,
{
    /// Commits the subscription change and its journal event together.
    async fn commit_event(
        &self,
        mut tx: S::Tx<'_>,
        event: SubEvent,
    ) -> RegistryDbResult<RecordedEvents> {
        self.applier
            .apply(&mut tx, &event, self.clock.now())
            .await?;
        let subscription = event.subscription().clone();
        let outcome = event.outcome_label();
        let event_type = event.event_type();
        let mut recorded_events = RecordedEvents::with_capacity(1);
        EventRecorder::new(&mut tx, &mut recorded_events, self.clock.as_ref())
            .record(event)
            .await?;
        tx.commit().await?;
        info!(
            subscriber_id = subscription.subscriber_id.as_str(),
            feed_url = subscription.feed_url.as_str(),
            outcome,
            %event_type,
            "registry subscription committed"
        );
        Ok(recorded_events)
    }
}

impl<S> CommandHandler<SubscribeFeedCommand> for SubHandler<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: SubscriptionDb,
{
    type Output = SubscribeFeedOutput;
    type Error = FeedRegistryError;

    async fn handle(
        &self,
        command: SubscribeFeedCommand,
    ) -> Result<HandledCommand<Self::Output>, Self::Error> {
        let (subscription, attrs) = command.into_parts(self.default_crawl_policy);
        let mut tx = self.db.begin().await?;
        let state = SubState::load(&mut tx, &subscription).await?;
        let (outcome, event) = state.subscribe(subscription, attrs);
        let recorded_events = self.commit_event(tx, event).await?;
        Ok(HandledCommand {
            output: SubscribeFeedOutput { outcome },
            recorded_events,
        })
    }
}

impl<S> CommandHandler<UnsubscribeFeedCommand> for SubHandler<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: SubscriptionDb,
{
    type Output = UnsubscribeFeedOutput;
    type Error = FeedRegistryError;

    async fn handle(
        &self,
        command: UnsubscribeFeedCommand,
    ) -> Result<HandledCommand<Self::Output>, Self::Error> {
        let subscription = command.into_subscription();
        let mut tx = self.db.begin().await?;
        let state = SubState::load(&mut tx, &subscription).await?;
        let (outcome, event) = state.unsubscribe(subscription)?;
        let recorded_events = self.commit_event(tx, event).await?;
        Ok(HandledCommand {
            output: UnsubscribeFeedOutput { outcome },
            recorded_events,
        })
    }
}
