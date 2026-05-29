#[derive(Debug, Clone, Copy)]
pub enum ReconcileTrigger {
    Startup,
    ScheduledTick,
    SubscriptionChanged,
    ManualRefreshRequested,
    PolicyChanged,
}

#[derive(Debug, Clone, Default)]
pub struct ReconcileOutcome {
    pub created: usize,
    pub updated: usize,
    pub noop: usize,
}
