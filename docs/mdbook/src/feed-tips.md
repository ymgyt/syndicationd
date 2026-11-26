# Feed tips

| What to follow             | How to get the feed                                    | Example                                                                      |
|----------------------------|--------------------------------------------------------|------------------------------------------------------------------------------|
| GitHub releases            | Append `/releases.atom` to the repository URL.         | [Syndicationd releases](https://github.com/ymgyt/syndicationd/releases.atom) |
| Git tags on GitHub         | Append `/tags.atom` to the repository URL.             | [Syndicationd tags](https://github.com/ymgyt/syndicationd/tags.atom)         |
| Versions of a Rust crate   | Use `https://static.crates.io/rss/crates/<crate>.xml`. | [synd releases](https://static.crates.io/rss/crates/synd.xml)                |
| Events in a connpass group | Append `/ja.atom` to the group URL.                    | [OpenTelemetry events](https://opentelemetry.connpass.com/ja.atom)           |
| Posts on a Hatena Blog     | Append `/feed` to the blog URL.                        | [Hatena Blog staff](https://staff.hatenablog.com/feed)                       |

The [crates.io feed announcement](https://blog.rust-lang.org/2024/07/29/crates-io-development-update/#rss-feeds) also lists registry-wide feeds for [new crates](https://static.crates.io/rss/crates.xml) and [new versions](https://static.crates.io/rss/updates.xml).

## Sites without a feed

For a blog without its own feed, look it up in [Olshansk/rss-feeds](https://github.com/Olshansk/rss-feeds).
Copy the URL from the **Feed** column into your reader, for example the [Anthropic Engineering feed](https://raw.githubusercontent.com/Olshansk/rss-feeds/main/feeds/feed_anthropic_engineering.xml).

To look up a page on [Open RSS](https://openrss.org/), insert `openrss.org/` after `https://` in its URL and open that address in your browser.
If a feed is available, copy the subscription URL from the preview.
For example, the [r/HelixEditor preview](https://openrss.org/www.reddit.com/r/HelixEditor) links to an [Atom feed for hot posts](https://openrss.org/feed/www.reddit.com/r/HelixEditor/hot/).
Reddit [retired its native `.rss` feeds in November 2026](https://support.reddithelp.com/hc/en-us/articles/55472297340308-Changelog-November-10-2026).
