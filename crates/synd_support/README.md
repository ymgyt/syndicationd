# synd-support

Shared support utilities for syndicationd crates.

This crate owns application-facing support code that is shared across product crates:

- filesystem, configuration, terminal color, byte, and humantime helpers
- feed excerpts and article extraction under `synd_support::html` (feature `html`)
- observability support under `synd_support::o11y`

Authentication remains in `synd-auth` because it has a separate security boundary.
