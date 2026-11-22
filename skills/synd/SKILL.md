---
name: synd
description: Use the synd CLI to list, add, remove, import, or export the user's RSS and Atom feed subscriptions.
---

# synd

Use the installed `synd` CLI. Running `synd` without a subcommand opens the
interactive reader, so use `synd feed` for subscription requests.

- **List subscriptions.** Run `synd feed export` and read the `feeds` array
  from its JSON output. Use the URLs in this array to identify subscriptions.
- **Subscribe.** Run `synd feed subscribe --url "<feed-url>"`. If the user
  specifies a category, pass `--category "<category>"`. For a priority, pass
  `--requirement` with one of `must`, `should`, or `may`.
- **Unsubscribe.** Run `synd feed unsubscribe --url "<feed-url>"` with the
  subscribed URL.
- **Export subscriptions.** Redirect `synd feed export` to the requested JSON
  file, for example `synd feed export > feeds.json`.
- **Import subscriptions.** Run `synd feed import feeds.json` for an exported
  file, or `synd feed import -` to read JSON from stdin. Report the final
  success count and any per-feed errors. An import can succeed for some feeds
  and fail for others while returning exit code zero.
