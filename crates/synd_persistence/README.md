# synd-persistence

Persistence adapters for syndicationd.

This crate owns concrete durable storage integrations. SQLite support is organized by adapter boundary: connection management is separate from the feed registry adapter, and registry-specific SQL lives under `sqlite/feed_registry`.

Domain policy and lifecycle decisions live in `synd-registry`; this crate persists and loads that state without owning the feed lifecycle.
