# synd-registry

Feed lifecycle registry for syndicationd.

This crate owns the desired-state model for subscribed feeds and turns it into refresh work. It defines subscription state, refresh policy, reconcile planning, in-memory refresh request coalescing, and the executor that fetches feeds and records durable state through a store trait.

The registry API is expressed as commands and queries: subscribe, unsubscribe,
manual refresh, reconcile, list subscriptions, list entries, and read refresh
status. Transaction boundaries remain visible through the store trait so callers
can reason about when subscription state, snapshots, and refresh state are
committed together.

Transport, authentication, terminal UI, and concrete database adapters live outside this crate.
