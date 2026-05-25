use std::collections::HashMap;

use chrono::Utc;

use crate::{
    error::FeedRegistryError,
    executor::RefreshExecutorHandle,
    model::{DesiredFeedRefresh, ReconcileOutcome, ReconcileTrigger, RefreshRequestDisposition},
    planner::RefreshPlanner,
    store::{FeedRegistryStore, RegistryTransaction},
};

#[derive(Clone)]
pub struct Reconciler<S> {
    store: S,
    executor: RefreshExecutorHandle,
}

impl<S> Reconciler<S>
where
    S: FeedRegistryStore,
{
    pub fn new(store: S, executor: RefreshExecutorHandle) -> Self {
        Self { store, executor }
    }

    pub async fn reconcile_now(
        &self,
        trigger: ReconcileTrigger,
    ) -> Result<ReconcileOutcome, FeedRegistryError> {
        let now = Utc::now();
        let mut tx = self.store.begin().await?;
        let subscriptions = tx.list_active_subscriptions().await?;
        let desired_feeds = DesiredFeedRefresh::from_subscriptions(subscriptions);
        let urls = desired_feeds
            .iter()
            .map(|feed| feed.feed_url.clone())
            .collect::<Vec<_>>();
        let states = tx.load_refresh_states(&urls).await?;
        tx.commit().await?;

        let state_by_url = states
            .into_iter()
            .map(|state| (state.feed_url.clone(), state))
            .collect::<HashMap<_, _>>();
        let plan = RefreshPlanner::plan_reconcile(desired_feeds, &state_by_url, trigger, now);

        let mut outcome = ReconcileOutcome {
            noop: plan.noop,
            ..ReconcileOutcome::default()
        };
        for intent in plan.intents {
            match self.executor.submit(intent).await.disposition {
                RefreshRequestDisposition::Created => outcome.created += 1,
                RefreshRequestDisposition::Promoted
                | RefreshRequestDisposition::CoalescedPending
                | RefreshRequestDisposition::JoinedRunning => outcome.updated += 1,
            }
        }

        Ok(outcome)
    }
}
