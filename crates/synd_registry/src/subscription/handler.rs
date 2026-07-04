use std::sync::Arc;

use chrono::{DateTime, Utc};
use synd_support::time::Clock;
use tracing::info;

use crate::{
    command::{
        SubscribeFeedCommand, SubscribeFeedOutput, UnsubscribeFeedCommand, UnsubscribeFeedOutput,
    },
    crawl::policy::CrawlPolicy,
    db::{CommitTx, FeedRegistryDb, SubscriptionStore},
    error::{FeedRegistryError, RegistryDbError, RegistryDbResult},
    event::{EventRecorder, RecordedEvents, SubEvent},
    handler::{CommandHandler, Decider, HandledCommand, StateApplier},
    subscription::{
        SubCommand, SubDecider, SubState, SubscribeOutcome, SubscriptionKey, UnsubscribeOutcome,
    },
};

/// Handles subscription commands as DB state changes plus journaled domain events.
#[derive(Clone)]
pub(crate) struct SubHandler<S> {
    db: S,
    default_crawl_policy: CrawlPolicy,
    clock: Arc<dyn Clock>,
    decider: SubDecider,
    applier: SubStateApplier,
}

impl<S> SubHandler<S> {
    pub(crate) fn new(db: S, default_crawl_policy: CrawlPolicy, clock: Arc<dyn Clock>) -> Self {
        Self {
            db,
            default_crawl_policy,
            clock,
            decider: SubDecider,
            applier: SubStateApplier,
        }
    }
}

impl SubState {
    /// Loads the current command-time state of one subscriber/feed relation.
    async fn load<Tx>(tx: &mut Tx, subscription: &SubscriptionKey) -> RegistryDbResult<Self>
    where
        Tx: SubscriptionStore + Send,
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

impl SubStateApplier {
    async fn apply_all<Tx>(
        &self,
        tx: &mut Tx,
        events: &[SubEvent],
        now: DateTime<Utc>,
    ) -> RegistryDbResult<()>
    where
        Tx: SubscriptionStore + Send,
    {
        for event in events {
            self.apply(tx, event, now).await?;
        }
        Ok(())
    }
}

impl<Tx> StateApplier<Tx> for SubStateApplier
where
    Tx: SubscriptionStore + Send,
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

/// Translates the events decided for one subscription command into the
/// command-specific output.
trait SubCommandOutput: Sized {
    const COMMAND_NAME: &'static str;

    fn from_event(event: &SubEvent) -> Result<Self, FeedRegistryError>;

    fn from_events(events: &[SubEvent]) -> Result<Self, FeedRegistryError> {
        match events {
            [event] => Self::from_event(event),
            events => Err(unexpected_event_count(Self::COMMAND_NAME, events.len())),
        }
    }
}

impl SubCommandOutput for SubscribeFeedOutput {
    const COMMAND_NAME: &'static str = "subscribe";

    fn from_event(event: &SubEvent) -> Result<Self, FeedRegistryError> {
        let outcome = match event {
            SubEvent::Subscribed(event) => SubscribeOutcome::Subscribed(event.subscription.clone()),
            SubEvent::Changed(event) => SubscribeOutcome::Changed(event.subscription.clone()),
            SubEvent::Unsubscribed(_) => {
                return Err(unexpected_event(Self::COMMAND_NAME, event));
            }
        };
        Ok(Self { outcome })
    }
}

impl SubCommandOutput for UnsubscribeFeedOutput {
    const COMMAND_NAME: &'static str = "unsubscribe";

    fn from_event(event: &SubEvent) -> Result<Self, FeedRegistryError> {
        let outcome = match event {
            SubEvent::Unsubscribed(event) => {
                UnsubscribeOutcome::Unsubscribed(event.subscription.clone())
            }
            SubEvent::Subscribed(_) | SubEvent::Changed(_) => {
                return Err(unexpected_event(Self::COMMAND_NAME, event));
            }
        };
        Ok(Self { outcome })
    }
}

impl<S> SubHandler<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: SubscriptionStore,
{
    /// Runs one subscription command through the shared decide -> apply ->
    /// record -> commit flow.
    async fn handle_sub_command<O>(
        &self,
        sub_command: SubCommand,
    ) -> Result<HandledCommand<O>, FeedRegistryError>
    where
        O: SubCommandOutput,
    {
        let mut tx = self.db.begin().await?;
        let state = SubState::load(&mut tx, sub_command.subscription()).await?;
        let events = self.decider.decide(sub_command, state)?;
        let output = O::from_events(&events)?;

        self.applier
            .apply_all(&mut tx, &events, self.clock.now())
            .await?;
        let mut recorded_events = RecordedEvents::with_capacity(events.len());
        EventRecorder::new(&mut tx, &mut recorded_events, self.clock.as_ref())
            .record_all(events.iter().cloned())
            .await?;
        tx.commit().await?;
        log_events(&events);

        Ok(HandledCommand {
            output,
            recorded_events,
        })
    }
}

impl<S> CommandHandler<SubscribeFeedCommand> for SubHandler<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: SubscriptionStore,
{
    type Output = SubscribeFeedOutput;
    type Error = FeedRegistryError;

    async fn handle(
        &self,
        command: SubscribeFeedCommand,
    ) -> Result<HandledCommand<Self::Output>, Self::Error> {
        let (subscription, attrs) = command.into_parts(self.default_crawl_policy);
        self.handle_sub_command(SubCommand::Subscribe {
            subscription,
            attrs,
        })
        .await
    }
}

impl<S> CommandHandler<UnsubscribeFeedCommand> for SubHandler<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: SubscriptionStore,
{
    type Output = UnsubscribeFeedOutput;
    type Error = FeedRegistryError;

    async fn handle(
        &self,
        command: UnsubscribeFeedCommand,
    ) -> Result<HandledCommand<Self::Output>, Self::Error> {
        self.handle_sub_command(SubCommand::Unsubscribe {
            subscription: command.into_subscription(),
        })
        .await
    }
}

fn unexpected_event_count(command_name: &'static str, event_count: usize) -> FeedRegistryError {
    RegistryDbError::internal_message(format!(
        "subscription {command_name} produced unexpected event count: {event_count}"
    ))
    .into()
}

fn unexpected_event(command_name: &'static str, event: &SubEvent) -> FeedRegistryError {
    RegistryDbError::internal_message(format!(
        "subscription {command_name} produced unexpected event type: {}",
        event.event_type()
    ))
    .into()
}

fn log_events(events: &[SubEvent]) {
    for event in events {
        let subscription = event.subscription();
        info!(
            subscriber_id = subscription.subscriber_id.as_str(),
            feed_url = subscription.feed_url.as_str(),
            outcome = event.outcome_label(),
            event_type = %event.event_type(),
            "registry subscription committed"
        );
    }
}
