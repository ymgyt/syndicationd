<div class="oranda-hide">

# Syndicationd

</div>

[![CI][workflow-ci-badge]][workflow-ci-url]
[![Release][workflow-release-badge]][workflow-release-url]
[![Audit][workflow-audit-badge]][workflow-audit-url]

[![Coverage][coverage-badge]][coverage-url]

[workflow-ci-badge]: https://img.shields.io/github/actions/workflow/status/ymgyt/syndicationd/ci.yaml?style=for-the-badge&logo=github&label=CI
[workflow-ci-url]: https://github.com/ymgyt/syndicationd/actions/workflows/ci.yaml
[workflow-release-badge]: https://img.shields.io/github/actions/workflow/status/ymgyt/syndicationd/release.yml?style=for-the-badge&logo=github&label=Release
[workflow-release-url]: https://github.com/ymgyt/syndicationd/actions/workflows/release.yml
[workflow-audit-badge]: https://img.shields.io/github/actions/workflow/status/ymgyt/syndicationd/audit.yaml?style=for-the-badge&logo=github&label=Audit
[workflow-audit-url]: https://github.com/ymgyt/syndicationd/actions/workflows/audit.yaml
[coverage-badge]: https://img.shields.io/codecov/c/github/ymgyt/syndicationd?token=W1A93WSPEE&style=for-the-badge&logo=codecov&color=brightgreen
[coverage-url]: https://app.codecov.io/github/ymgyt/syndicationd
![Demo](https://raw.githubusercontent.com/ymgyt/syndicationd/main/etc/demo/demo.gif)

`synd` is a terminal feed reader for RSS and Atom.

It lets you subscribe to feeds, browse entries, filter them by priority,
category, and keyword, and open entries in your browser.

Subscriptions, entries, and reading state are stored in SQLite by default.

This repository is named `syndicationd`. The terminal application is installed
as `synd`.

**Table of Contents:**

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Usage](#usage)
  - [Keymap](#keymap)
  - [Subscribe Feed](#subscribe-feed)
  - [Edit or Unsubscribe Feed](#edit-or-unsubscribe-feed)
  - [Filter Feeds and Entries](#filter-feeds-and-entries)
  - [Open Feed Entry](#open-feed-entry)
  - [Import and Export Feeds](#import-and-export-feeds)
  - [Clean](#remove-cache-and-logs)
- [Configuration](#configuration)
- [Documentation](#documentation)
- [Advanced](#advanced)
- [Development](#development)
- [Project Goals](#project-goals)
- [Feed Tips](#feed-tips)
- [License](#license)


## Installation

Install from crates.io:

```sh
cargo install synd --locked
```

Other package managers, installers, and pre-built binaries are also supported.

<details>
<summary>Show all installation methods</summary>

### nix

```sh
nix profile install github:ymgyt/syndicationd#synd
```

### arch linux

```sh
pacman -S syndicationd
```

### brew

```sh
brew install ymgyt/homebrew-syndicationd/synd
```

### shell

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ymgyt/syndicationd/releases/latest/download/synd-installer.sh | sh
```

### npm

```sh
npm install @syndicationd/synd
```

### powershell

```sh
powershell -c "irm https://github.com/ymgyt/syndicationd/releases/latest/download/synd-installer.ps1 | iex"
```

### docker

```sh
docker run -it ghcr.io/ymgyt/synd
```

### pre-built binaries

Pre-built binaries are available in [GitHub releases](https://github.com/ymgyt/syndicationd/releases).


### source

```sh
cargo install --git https://github.com/ymgyt/syndicationd/ synd
```

Use the source install to try the current `main` branch behavior before the next release.

</details>

> [!NOTE]
> `synd` requires [Nerd Fonts](https://www.nerdfonts.com/) to be installed on your system for rendering icons.

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

## Usage

Run `synd --help` to see the full command and option list.

`synd` lets you:

* subscribe to RSS/Atom feeds
* open entries in your web browser or a text browser
* filter feeds and entries by priority, category, and keyword

### Keymap

Basic keys:

* `Tab`: switch tabs
* `a`: add a feed on the Feeds tab
* `Enter`: open the selected entry
* `Space`: open the selected entry with the configured text browser command
* `/`: search entries
* `q`: quit

See [docs/mdbook/src/keymap.md](./docs/mdbook/src/keymap.md) for the full
keymap and custom key binding syntax.

### Subscribe Feed

To subscribe to a feed, switch to the Feeds tab with `Tab`, then press `a`.
`synd` uses [edit](https://docs.rs/edit/latest/edit/) to launch your editor.
Enter the feed in this format:

`Requirement` `Category` `URL`

When you close the editor, the feed is saved to the local SQLite database by
default.

#### Requirement

`Requirement` is the priority of the feed.

It uses the names `MUST`, `SHOULD`, and `MAY` by analogy with
[RFC2119](https://datatracker.ietf.org/doc/html/rfc2119).

It can be one of:

* `MUST`: most important
* `SHOULD`: normal priority
* `MAY`: low priority

#### Category

`Category` represents the category of the feed. You can specify any value.
The values that `synd` recognizes as categories are defined in
[`categories.toml`](./categories.toml). Default values and additional
categories can be added from the configuration file.

### Edit or Unsubscribe Feed

To change the requirement or category of a feed, select it in the Feeds tab and
press `e`.

To unsubscribe from a feed, select it and press `d`.

### Filter Feeds and Entries

Feeds and entries can be filtered as follows.

#### By requirement

To filter by requirement, press `h` or `l`.
If the filter is `MUST`, only `MUST` feeds and entries are displayed.
If it is `SHOULD`, `MUST` and `SHOULD` feeds and entries are displayed.
If it is `MAY`, all feeds and entries are displayed.

#### By categories

To filter by category, press `c`. This shows keys for toggling each category.
Press `-` to deactivate all categories. Press `+` to activate all categories.

You can exit category filter mode by pressing `Esc`.
The icons for categories can be specified in `categories.toml`.

### Open Feed Entry

To open a feed entry in a web browser, select the entry and press `Enter`.
To view the entry in a text browser within the terminal, press `Space`.
The command used by `Space` can be specified with the `$SYND_BROWSER`
environment variable, or through related flags or configuration files.
The command is executed as `$SYND_BROWSER $SYND_BROWSER_ARGS <entry url>`.

### Import and Export Feeds

Export subscriptions as JSON and import the same format:

```console
synd feed export > feeds.json
synd feed import feeds.json
```

Print the JSON schema when needed:

```console
synd feed export --print-schema
synd feed import --print-schema
```

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

## Configuration

Configuration can be set with flags, environment variables, or a config file.

See [docs/mdbook/src/configuration.md](./docs/mdbook/src/configuration.md) for
config file locations, available settings, and keymap customization.

## Documentation

* [Configuration](./docs/mdbook/src/configuration.md)
* [Keymap](./docs/mdbook/src/keymap.md)
* [GitHub Notifications](./docs/github-notifications.md)

## Advanced

### GitHub Notifications

GitHub notification support is optional and separate from the feed reader
workflow. See [docs/github-notifications.md](./docs/github-notifications.md)
for setup, keymap, and token scope details.

## Development

Please refer to [CONTRIBUTING.md](/CONTRIBUTING.md) to get started with development.

## Project Goals

* **Terminal-first feed reader**.
  Build a feed reader for terminal users. No recommendations, no curation, no behavior analysis.

* **Local storage by default**.
  Store subscriptions and feed state in SQLite by default.

* **Long-term maintenance**.
  Prefer simple designs and stable dependencies so the project can be maintained for years.

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
