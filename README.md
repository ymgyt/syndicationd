<div class="oranda-hide">

# Syndicationd

</div>

[![CI][workflow-ci-badge]][workflow-ci-url]
[![Release][workflow-release-badge]][workflow-release-url]
[![Audit][workflow-audit-badge]][workflow-audit-url]

[![Coverage][coverage-badge]][coverage-url]

[crates-badge]: https://img.shields.io/crates/v/synd-term?style=for-the-badge&logo=rust
[crates-url]: https://crates.io/crates/synd-term
[license-badge]: https://img.shields.io/badge/license-MIT%2FApache--2.0-blue?style=for-the-badge
[workflow-ci-badge]: https://img.shields.io/github/actions/workflow/status/ymgyt/syndicationd/ci.yaml?style=for-the-badge&logo=github&label=CI
[workflow-ci-url]: https://github.com/ymgyt/syndicationd/actions/workflows/ci.yaml
[workflow-release-badge]: https://img.shields.io/github/actions/workflow/status/ymgyt/syndicationd/release.yml?style=for-the-badge&logo=github&label=Release
[workflow-release-url]: https://github.com/ymgyt/syndicationd/actions/workflows/release.yml
[workflow-audit-badge]: https://img.shields.io/github/actions/workflow/status/ymgyt/syndicationd/audit.yaml?style=for-the-badge&logo=github&label=Audit
[workflow-audit-url]: https://github.com/ymgyt/syndicationd/actions/workflows/audit.yaml
[coverage-badge]: https://img.shields.io/codecov/c/github/ymgyt/syndicationd?token=W1A93WSPEE&style=for-the-badge&logo=codecov&color=brightgreen
[coverage-url]: https://app.codecov.io/github/ymgyt/syndicationd
![Demo](https://raw.githubusercontent.com/ymgyt/syndicationd/main/etc/demo/demo.gif)

Syndicationd provides `synd`, a terminal feed reader for RSS and Atom.

It is designed for browsing and managing feeds from the TUI, with subscriptions
stored in SQLite by default.

**Table of Contents:**

- [Installation](#installation)
- [Overview](#overview)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Usage](#usage)
  - [Keymap](#keymap)
  - [Subscribe Feed](#subscribe-feed)
  - [Edit or Unsubscribe Feed](#edit-or-unsubscribe-feed)
  - [Filter Feeds and Entries](#filter-feeds-and-entries)
  - [Open Feed Entry](#open-feed-entry)
  - [Import and Export Feeds](#import-and-export-feeds)
  - [Theme](#theme)
  - [Log](#log)
  - [Clean](#remove-cache-and-logs)
- [Advanced](#advanced)
- [Development](#development)
- [Project Goals](#project-goals)
- [Feed Tips](#feed-tips)
- [License](#license)


## Installation

Install from crates.io:

```sh
cargo install synd-term --locked
```

Other package managers, installers, and pre-built binaries are also supported.

<details>
<summary>Show all installation methods</summary>

### nix

```sh
nix profile install github:ymgyt/syndicationd/synd-term-v0.3.2
```

### arch linux

```sh
pacman -S syndicationd
```

### brew

```sh
brew install ymgyt/homebrew-syndicationd/synd-term
```

### shell

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ymgyt/syndicationd/releases/download/synd-term-v0.3.2/synd-term-installer.sh | sh
```

### npm

```sh
npm install @syndicationd/synd-term
```

### powershell

```sh
powershell -c "irm https://github.com/ymgyt/syndicationd/releases/download/synd-term-v0.3.2/synd-term-installer.ps1 | iex"
```

### docker

```sh
docker run -it ghcr.io/ymgyt/synd-term
```

### pre-built binaries

Pre-built binaries are available in [GitHub releases](https://github.com/ymgyt/syndicationd/releases).


### source

```sh
cargo install --git https://github.com/ymgyt/syndicationd/ synd-term
```

Use the source install to try the current `main` branch behavior before the next release.

</details>

> [!NOTE]
> `synd` requires [Nerd Fonts](https://www.nerdfonts.com/) to be installed on your system for rendering icons.

## Overview

`synd` is built around the normal terminal workflow:

* subscribe to RSS/Atom feeds
* open entries in your web browser or a text browser
* filter feeds and entries by requirement, category, and keyword

## Quick Start

Start the TUI:

```sh
synd
```

To add a feed, switch to the Feeds tab with `Tab`, press `a`, and enter one
subscription line in the editor:

```text
MUST rust https://this-week-in-rust.org/atom.xml
```

Select an entry and press `Enter` to open it in your web browser. Press `Space`
to open it with the configured text browser command.

## Configuration

Settings can be configured in the following ways, in order of priority:

* Command line flag
* Environment variables
* Configuration file
* Default value

The location of the configuration file can be specified using `--config` or the environment variable `SYND_CONFIG_FILE`.
By default, `synd` searches the following locations depending on the platform:

| Platform | Locations |
| ---      | ---       |
| Linux    | `$XDG_CONFIG_HOME/syndicationd/config.toml`<br>`$HOME/.config/syndicationd/config.toml` |
| macOS    | `$HOME/Library/Application Support/syndicationd/config.toml` |
| Windows  | `{FOLDERID_RoamingAppData}/syndicationd/config.toml` |

`synd` does not automatically create configuration files.
When creating a configuration file, you can use the following command:

```sh
synd config init > config.toml
```

To inspect the resolved configuration after flags, environment variables,
configuration files, and defaults have been applied, use `config view`.

```sh
synd config view
synd config view -o json
```

### Common Settings

| Flag               | Environment variable     | Configuration file       | Default                             | Description                                         |
|--------------------|--------------------------|--------------------------|-------------------------------------|-----------------------------------------------------|
| `--config`         | `SYND_CONFIG_FILE`       | \-                       | see [configuration](#configuration) | Configuration file path                             |
| `--log`            | `SYND_LOG_FILE`          | `[log.path]`             | see `synd config view`              | Log file path                                       |
| `--cache-dir`      | `SYND_CACHE_DIR`         | `[cache.directory]`      | see `synd config view`              | Cache directory                                     |
| `--theme`          | `SYND_THEME`             | `[theme.name]`           | `ferra`                             | Theme name                                          |
| `--sqlite-db`      | `SYND_SQLITE_DB`         | `[backend.sqlite_db]`    | see `synd config view`              | SQLite database path                                |
| `--entries-limit`  | `SYND_ENTRIES_LIMIT`     | `[feed.entries_limit]`   | `200`                               | Feed entries to fetch                               |
| `--browser`        | `SYND_BROWSER`           | `[feed.browser.command]` | \-                                  | Command to browse feed                              |
| `--browser-args`   | `SYND_BROWSER_ARGS`      | `[feed.browser.args]`    | `[]`                                | Command args to browse feed                         |

Optional integrations are documented under [Advanced](#advanced).

### Additional categories

To add a category, add the following content to the configuration file:

```toml
[categories.rust]
icon = { symbol = "🦀", color = { rgb = 0xF74C00 } }
aliases = ["rs"]
```

## Usage

Run `synd --help` to see the full command and option list.

### Keymap

| Key     | Description                                   |
| ---     | ---                                           |
| `k/j`   | Move up/down                                  |
| `gg`    | Go to first                                   |
| `ge`    | Go to end                                     |
| `Tab`   | Switch Tab                                    |
| `Enter` | Open entry/feed with web browser              |
| `Space` | Open entry with text browser(`$SYND_BROWSER`) |
| `a`     | Add feed subscription(on Feeds Tab)           |
| `e`     | Edit subscribed feed(on Feeds Tab)            |
| `d`     | Delete subscribed feed(on Feeds Tab)          |
| `r`     | Reload entries/feeds                          |
| `h/l`   | Change requirement filter                     |
| `c`     | Activate category filter(Esc to deactivate)   |
| `+`     | Activate all category(on Category filter)     |
| `-`     | Deactivate all category(on Category filter)   |
| `/`     | Activate keyword search(Esc to deactivate)    |
| `q`     | Quit app                                      |

For more details, refer to [`keymap/default.rs`](https://github.com/ymgyt/syndicationd/blob/main/crates/synd_term/src/keymap/default.rs).

### Subscribe feed

To subscribe a feed, type "Tab" to move to Feeds tab and then press "a".
`synd` uses [edit](https://docs.rs/edit/latest/edit/) to launch the user's editor(like a git commit).
The feed to subscribe to should be entered in the format:
`Requirement` `Category` `URL`

When you close the editor, the feed is saved to the local SQLite database by
default.

The command-line shape for direct subscription management is reserved as
`synd feed subscribe` and `synd feed unsubscribe`, but these commands currently
return `not_yet_implemented` while the client structure is being reworked.

#### Requirement

`Requirement` indicates the importance of the feed.
This uses an analogy to [RFC2119](https://datatracker.ietf.org/doc/html/rfc2119) and can take one of the following values:

* `MUST`: Most important, must be read.
* `SHOULD`: Next in importance, should be read unless there is a special reason not to.
* `MAY`: Lowest importance, may be read.

#### Category

`Category` represents the category of the feed. You can specify any value as a category. The values that `synd` recognizes as categories are defined in [`categories.toml`](./categories.toml). Default values and additional categories can be added from the configuration file.


### Edit or Unsubscribe Feed

To change the requirement or category of a feed you have already subscribed to, select the target feed in the Feeds tab and then press "e".

To unsubscribe from a feed, select the target feed and press "d".

### Filter Feeds and Entries

Feeds and entries can be filtered as follows.

#### By requirement

To filter based on the specified requirement, press "h/l(Left/Right)".
If you set the filter to `MUST`, only those marked as MUST will be displayed. Setting it to SHOULD will display feeds and entries marked as MUST and SHOULD. If set to MAY, all feeds and entries will be displayed.

#### By categories

To filter based on categories, press "c". This will display a label with keys to control the activation/deactivation of each category, allowing you to toggle the visibility of categories.
Pressing "-" will deactivate all categories, and pressing "+" will activate all categories.

You can exit the filter category mode by pressing the "Esc" key.
The icons for categories can be specified in `categories.toml`.

### Open feed entry

To open a feed entry in a web browser, select the entry and press Enter.
To view the entry in a text browser within the terminal, press `Space`.
The command that is triggered by pressing the Space can be specified using the `$SYND_BROWSER` environment variable, or through related flags or configuration files.
The command is executed as `$SYND_BROWSER $SYND_BROWSER_ARGS <entry url>`.

### Import and Export Feeds

To export subscribed feeds from the local database, execute the
`synd feed export` command.

```sh
synd feed export > feeds.json
```

You can check the JSON schema of the data to be exported with
`synd feed export --print-schema`.

You can subscribe to multiple feeds at once using the `synd feed import`
command. By default this imports into the local SQLite database.

The input schema is the same as that of `synd feed export`. You can also check
it with `synd feed import --print-schema`.

```sh
# from stdin
echo '{"feeds": [ {"url": "https://this-week-in-rust.org/atom.xml", "category": "rust", "requirement": "Must" } ]}' \
  | synd feed import -

# read from file
synd feed export > feeds.json
synd feed import feeds.json
```

Because export and import use the same JSON document, this also works:

```sh
synd feed export | synd feed import -
```

### Theme

The theme can be changed using the `--theme` flag. Please refer to the help for the values that can be specified.

### Log

The default log file path is based on [`ProjectDirs::data_dir()`](https://docs.rs/directories/latest/directories/struct.ProjectDirs.html#method.data_dir).
Use `synd config view` to inspect the resolved output destination.

You can modify the [log directives](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives) using the environment variable `SYND_LOG`. (for example, `SYND_LOG=synd=debug`)

### Remove cache and logs

`synd clean` removes known cache files and logs. Use `--cache` or `--logs` to
limit the target.

```sh
synd clean
synd clean --cache
synd clean --logs
```

The cache directory itself is preserved, and only known cache files are removed.
`synd clean` does not remove the local SQLite database. Database operations are
handled separately from cache/log cleanup.

## Advanced

### Local Data

The local database path can be set with `--sqlite-db`, `SYND_SQLITE_DB`, or
`[backend.sqlite_db]`.

```sh
synd --sqlite-db ~/.local/share/syndicationd/synd.db
```

### GitHub Notifications

GitHub notification support is optional and separate from the feed reader
workflow. See [docs/github-notifications.md](./docs/github-notifications.md)
for setup, keymap, and token scope details.

## Development

Please refer to [CONTRIBUTING.md](/CONTRIBUTING.md) to get started with development.

## Project Goals

* **A terminal-first feed reader**. Create a simple feed reader for terminal users that does not involve curation, recommendations, or user behavior analysis.

* **Local by default**. Keep subscriptions and feed state in a local SQLite database.

* **Longevity**. Maintain this project for as long as possible, with a minimum maintenance period of at least 5 years.

## Feed Tips

A few sources expose useful feeds:

* Add [`openrss.org/`](https://openrss.org/) to the beginning of the URL to get its RSS feed. for example, for `https://example.ymgyt.io`, it would be `https://openrss.org/example.ymgyt.io`

* You can retrieve various updates as feeds on GitHub.
  * To obtain releases of a repository, specify `releases.atom`. for example, to obtain releases of syndicationd, specify `https://github.com/ymgyt/syndicationd/releases.atom`
  * For tags, it's `https://github.com/ymgyt/syndicationd/tag.atom`

* crates.io has introduced a couple of experimental [RSS feeds](https://blog.rust-lang.org/2024/07/29/crates-io-development-update.html#rss-feeds)

* Adding `.rss` to the end of a Reddit URL allows you to retrieve the feed. for example, for `https://www.reddit.com/r/HelixEditor/`, it would be `https://www.reddit.com/r/HelixEditor.rss`

## License

This project is available under the terms of either the [Apache 2.0 license](./LICENSE-APACHE) or the [MIT license](./LICENSE-MIT).
