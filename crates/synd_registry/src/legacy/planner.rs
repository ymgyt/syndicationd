use std::collections::HashMap;

use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;

use super::model::{
    DesiredFeedRefresh, NewRefreshRequest, ReconcileTrigger, RefreshIntent, RefreshIntentKind,
    RefreshInterval, RefreshRequest, RefreshRequestDisposition, RefreshRequestStatus,
    RefreshRequestUpdate, RefreshSchedule, RefreshState,
};

#[derive(Debug, Clone)]
pub enum RefreshRequestDecision {
    Create(NewRefreshRequest),
    Promote(RefreshRequestUpdate),
    MergePending(RefreshRequestUpdate),
    JoinRunning(RefreshRequestUpdate),
}

impl RefreshRequestDecision {
    pub fn disposition(&self) -> RefreshRequestDisposition {
        match self {
            Self::Create(_) => RefreshRequestDisposition::Created,
            Self::Promote(_) => RefreshRequestDisposition::Promoted,
            Self::MergePending(_) => RefreshRequestDisposition::CoalescedPending,
            Self::JoinRunning(_) => RefreshRequestDisposition::JoinedRunning,
        }
    }
}

pub struct RefreshRequestPolicy;

impl RefreshRequestPolicy {
    pub fn coalesce(
        incoming: RefreshIntent,
        active: Option<RefreshRequest>,
    ) -> RefreshRequestDecision {
        let Some(active) = active else {
            return RefreshRequestDecision::Create(incoming.into());
        };

        match active.status {
            RefreshRequestStatus::Pending if incoming.priority > active.priority => {
                RefreshRequestDecision::Promote(RefreshRequestUpdate::from_merge(
                    &active, &incoming,
                ))
            }
            RefreshRequestStatus::Pending => RefreshRequestDecision::MergePending(
                RefreshRequestUpdate::from_merge(&active, &incoming),
            ),
            RefreshRequestStatus::Running => RefreshRequestDecision::JoinRunning(
                RefreshRequestUpdate::from_merge(&active, &incoming),
            ),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReconcilePlan {
    pub intents: Vec<RefreshIntent>,
    pub noop: usize,
}

pub struct RefreshPlanner;

impl RefreshPlanner {
    pub fn plan_reconcile(
        desired_feeds: Vec<DesiredFeedRefresh>,
        state_by_url: &HashMap<FeedUrl, RefreshState>,
        trigger: ReconcileTrigger,
        now: DateTime<Utc>,
    ) -> ReconcilePlan {
        let mut plan = ReconcilePlan::default();
        for desired_feed in desired_feeds {
            if Self::is_due(
                &desired_feed,
                state_by_url.get(&desired_feed.feed_url),
                now,
                trigger,
            ) {
                plan.intents.push(RefreshIntent::new(
                    desired_feed.feed_url,
                    Self::intent_kind(trigger),
                    None,
                    now,
                ));
            } else {
                plan.noop = plan.noop.saturating_add(1);
            }
        }
        plan
    }

    fn intent_kind(trigger: ReconcileTrigger) -> RefreshIntentKind {
        match trigger {
            ReconcileTrigger::Startup => RefreshIntentKind::Startup,
            ReconcileTrigger::ScheduledTick => RefreshIntentKind::Scheduled,
            ReconcileTrigger::SubscriptionChanged => RefreshIntentKind::Initial,
            ReconcileTrigger::ManualRefreshRequested => RefreshIntentKind::Manual,
            ReconcileTrigger::PolicyChanged => RefreshIntentKind::PolicyChanged,
        }
    }

    fn is_due(
        desired_feed: &DesiredFeedRefresh,
        state: Option<&RefreshState>,
        now: DateTime<Utc>,
        trigger: ReconcileTrigger,
    ) -> bool {
        if matches!(
            trigger,
            ReconcileTrigger::SubscriptionChanged
                | ReconcileTrigger::ManualRefreshRequested
                | ReconcileTrigger::PolicyChanged
        ) {
            return true;
        }

        match desired_feed.policy.schedule {
            RefreshSchedule::Manual => false,
            RefreshSchedule::Interval(interval) => {
                let next = state
                    .and_then(|state| state.next_refresh_after)
                    .or_else(|| {
                        state
                            .and_then(|state| state.last_success_at)
                            .map(|last| add_duration(last, interval))
                    });
                next.is_none_or(|next| next <= now)
            }
        }
    }
}

fn add_duration(time: DateTime<Utc>, interval: RefreshInterval) -> DateTime<Utc> {
    chrono::Duration::from_std(interval.duration()).map_or(time, |duration| time + duration)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::Utc;
    use synd_feed::types::FeedUrl;

    use super::*;
    use crate::legacy::model::{
        EffectiveRefreshPolicy, RefreshIntentKind, RefreshPolicy, RefreshPriority,
        RefreshRequestId, SubscriberId,
    };

    fn url() -> FeedUrl {
        FeedUrl::parse("https://example.com/feed.xml").unwrap()
    }

    fn interval(duration: Duration) -> RefreshInterval {
        RefreshInterval::try_from(duration).unwrap()
    }

    fn pending(priority: RefreshPriority) -> RefreshRequest {
        let now = Utc::now();
        RefreshRequest {
            id: RefreshRequestId::new("req-1"),
            feed_url: url(),
            intent: RefreshIntentKind::Scheduled,
            priority,
            requested_by: None,
            requested_at: Some(now),
            signal_count: 1,
            not_before: now,
            status: RefreshRequestStatus::Pending,
            attempt_count: 0,
            lease_until: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn manual() -> RefreshIntent {
        let now = Utc::now();
        RefreshIntent::new(
            url(),
            RefreshIntentKind::Manual,
            Some(SubscriberId::new("local")),
            now,
        )
    }

    #[test]
    fn manual_creates_without_active_request() {
        assert!(matches!(
            RefreshRequestPolicy::coalesce(manual(), None),
            RefreshRequestDecision::Create(_)
        ));
    }

    #[test]
    fn manual_promotes_pending_background_request() {
        assert!(matches!(
            RefreshRequestPolicy::coalesce(manual(), Some(pending(RefreshPriority::Background)),),
            RefreshRequestDecision::Promote(_)
        ));
    }

    #[test]
    fn repeated_manual_merges_pending_interactive_request() {
        assert!(matches!(
            RefreshRequestPolicy::coalesce(manual(), Some(pending(RefreshPriority::Interactive)),),
            RefreshRequestDecision::MergePending(_)
        ));
    }

    #[test]
    fn manual_joins_running_request() {
        let mut request = pending(RefreshPriority::Background);
        request.status = RefreshRequestStatus::Running;

        assert!(matches!(
            RefreshRequestPolicy::coalesce(manual(), Some(request)),
            RefreshRequestDecision::JoinRunning(_)
        ));
    }

    fn desired_feed(refresh_policy: RefreshPolicy) -> DesiredFeedRefresh {
        DesiredFeedRefresh {
            feed_url: url(),
            policy: EffectiveRefreshPolicy {
                schedule: refresh_policy.schedule,
            },
        }
    }

    #[test]
    fn scheduled_reconcile_skips_manual_policy() {
        let plan = RefreshPlanner::plan_reconcile(
            vec![desired_feed(RefreshPolicy {
                schedule: RefreshSchedule::Manual,
            })],
            &HashMap::new(),
            ReconcileTrigger::ScheduledTick,
            Utc::now(),
        );

        assert!(plan.intents.is_empty());
        assert_eq!(plan.noop, 1);
    }

    #[test]
    fn scheduled_reconcile_creates_intent_when_refresh_is_due() {
        let plan = RefreshPlanner::plan_reconcile(
            vec![desired_feed(RefreshPolicy::interval(interval(
                Duration::from_mins(1),
            )))],
            &HashMap::new(),
            ReconcileTrigger::ScheduledTick,
            Utc::now(),
        );

        assert_eq!(plan.intents.len(), 1);
        assert_eq!(plan.noop, 0);
        assert_eq!(plan.intents[0].intent, RefreshIntentKind::Scheduled);
    }
}
