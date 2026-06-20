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

/// Applies subscription domain events to the subscription store transaction.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SubStateApplier;

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
        let sub_command = SubCommand::Subscribe {
            subscription: subscription.clone(),
            attrs,
        };

        let mut tx = self.db.begin().await?;
        let state = load_state(&mut tx, &subscription).await?;
        let events = self.decider.decide(sub_command, state)?;
        let output = subscribe_output(&events)?;

        apply_events(&self.applier, &mut tx, &events, self.clock.now()).await?;
        let recorded_events = record_events(&mut tx, &events, self.clock.as_ref()).await?;
        tx.commit().await?;
        log_events(&events);

        Ok(HandledCommand {
            output,
            recorded_events,
        })
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
        let subscription = command.into_subscription();
        let sub_command = SubCommand::Unsubscribe {
            subscription: subscription.clone(),
        };

        let mut tx = self.db.begin().await?;
        let state = load_state(&mut tx, &subscription).await?;
        let events = self.decider.decide(sub_command, state)?;
        let output = unsubscribe_output(&events)?;

        apply_events(&self.applier, &mut tx, &events, self.clock.now()).await?;
        let recorded_events = record_events(&mut tx, &events, self.clock.as_ref()).await?;
        tx.commit().await?;
        log_events(&events);

        Ok(HandledCommand {
            output,
            recorded_events,
        })
    }
}

async fn load_state<Tx>(
    tx: &mut Tx,
    subscription: &SubscriptionKey,
) -> Result<SubState, FeedRegistryError>
where
    Tx: SubscriptionStore + Send,
{
    if tx
        .has_subscription(&subscription.subscriber_id, &subscription.feed_url)
        .await?
    {
        Ok(SubState::Subscribed)
    } else {
        Ok(SubState::NotSubscribed)
    }
}

async fn apply_events<Tx>(
    applier: &SubStateApplier,
    tx: &mut Tx,
    events: &[SubEvent],
    now: DateTime<Utc>,
) -> RegistryDbResult<()>
where
    Tx: SubscriptionStore + Send,
{
    for event in events {
        applier.apply(tx, event, now).await?;
    }
    Ok(())
}

async fn record_events<Tx>(
    tx: &mut Tx,
    events: &[SubEvent],
    clock: &(dyn Clock + '_),
) -> RegistryDbResult<RecordedEvents>
where
    Tx: crate::event::EventJournalAppend + Send,
{
    let mut recorded_events = RecordedEvents::with_capacity(events.len());
    {
        let mut recorder = EventRecorder::new(tx, &mut recorded_events, clock);
        recorder.record_all(events.iter().cloned()).await?;
    }
    Ok(recorded_events)
}

fn subscribe_output(events: &[SubEvent]) -> Result<SubscribeFeedOutput, FeedRegistryError> {
    let event = exactly_one(events, "subscribe")?;

    let outcome = match event {
        SubEvent::Subscribed(event) => SubscribeOutcome::Subscribed(event.subscription.clone()),
        SubEvent::Changed(event) => SubscribeOutcome::Changed(event.subscription.clone()),
        SubEvent::Unsubscribed(_) => return Err(unexpected_event("subscribe", event)),
    };
    Ok(SubscribeFeedOutput { outcome })
}

fn unsubscribe_output(events: &[SubEvent]) -> Result<UnsubscribeFeedOutput, FeedRegistryError> {
    let event = exactly_one(events, "unsubscribe")?;

    let outcome = match event {
        SubEvent::Unsubscribed(event) => {
            UnsubscribeOutcome::Unsubscribed(event.subscription.clone())
        }
        SubEvent::Subscribed(_) | SubEvent::Changed(_) => {
            return Err(unexpected_event("unsubscribe", event));
        }
    };
    Ok(UnsubscribeFeedOutput { outcome })
}

fn exactly_one<'a>(
    events: &'a [SubEvent],
    command_name: &'static str,
) -> Result<&'a SubEvent, FeedRegistryError> {
    match events {
        [] => Err(unexpected_event_count(command_name, 0)),
        [event] => Ok(event),
        events => Err(unexpected_event_count(command_name, events.len())),
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
