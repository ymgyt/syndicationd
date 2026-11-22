<div class="oranda-hide">

# Syndicationd

</div>

[![CI][workflow-ci-badge]][workflow-ci-url]
[![Release][workflow-release-badge]][workflow-release-url]
[![Audit][workflow-audit-badge]][workflow-audit-url]
[![Coverage][coverage-badge]][coverage-url]

![Demo](https://raw.githubusercontent.com/ymgyt/syndicationd/main/etc/demo/demo.gif)

`synd` is an RSS and Atom feed reader with a TUI and a CLI.

## Installation

Install from crates.io:

```sh
cargo install synd --version 0.4.0 --locked
```

<details>
<summary>Other installation methods</summary>

### nix

```sh
nix profile add github:ymgyt/syndicationd/v0.4.0#synd
```

### homebrew

```sh
brew install ymgyt/syndicationd/synd
```

### shell

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ymgyt/syndicationd/releases/download/v0.4.0/synd-installer.sh | sh
```

### npm

```sh
npm install @syndicationd/synd@0.4.0
```

### powershell

```sh
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ymgyt/syndicationd/releases/download/v0.4.0/synd-installer.ps1 | iex"
```

### docker

```sh
docker run --rm -it ghcr.io/ymgyt/synd:0.4.0
```

### pre-built binaries

Pre-built binaries are available in the
[`v0.4.0` GitHub release](https://github.com/ymgyt/syndicationd/releases/tag/v0.4.0).

### source

```sh
cargo install --git https://github.com/ymgyt/syndicationd/ synd
```

Use the source install to try the current `main` branch behavior before the next release.

</details>

> [!NOTE]
> Install a [Nerd Font](https://www.nerdfonts.com/) to display icons in the TUI.

## Quick Start

Run `synd` to open the TUI. Press `Tab` to switch to Feeds, then `a` to open
your editor. Add the following line to subscribe to This Week in Rust.

```text
MUST rust https://this-week-in-rust.org/atom.xml
```

Save and close the editor, then return to the Entries tab. Once the feed has
loaded, select an entry and press `Enter` to open it in your web browser.

## TUI

- Customizable [Helix-like keybindings][keymap]
- Keyword search and category filters for feeds and entries
- Open entries in a web or text browser

## CLI

Subscribe to This Week in Rust with the same category and priority as in the Quick Start.

```sh
synd feed subscribe --url https://this-week-in-rust.org/atom.xml --category rust --requirement must
```

Save your subscriptions to a JSON file.

```sh
synd feed export > feeds.json
```

Import the subscriptions from that file.

```sh
synd feed import feeds.json
```

`synd config view -o json` prints the effective configuration as JSON.
`synd doctor` checks local paths and reports the status of the daemon.

Run `synd --help` to explore the other commands. For settings and configuration file
locations, see the [configuration guide][configuration].

[keymap]: https://github.com/ymgyt/syndicationd/blob/main/docs/mdbook/src/keymap.md
[configuration]: https://github.com/ymgyt/syndicationd/blob/main/docs/mdbook/src/configuration.md

## Agent skill

With `synd` installed, your agent can use the `synd` skill to manage feed
subscriptions.

### [skills](https://github.com/vercel-labs/skills)

```sh
npx skills add -g ymgyt/syndicationd
```

### [APM](https://github.com/microsoft/apm)

```sh
apm install -g ymgyt/syndicationd
```

## Development

See
[CONTRIBUTING.md](https://github.com/ymgyt/syndicationd/blob/main/CONTRIBUTING.md)
to get started with development.

## Project Goals

* **Terminal-first feed reader**.
  Build a feed reader for terminal users. No recommendations, no curation, no behavior analysis.

* **Local storage by default**.
  Store subscriptions and feed state in SQLite by default.

* **Long-term maintenance**.
  Prefer simple designs and stable dependencies so the project can be maintained for years.

## License

This project is available under the terms of either the
[Apache 2.0 license](https://github.com/ymgyt/syndicationd/blob/main/LICENSE-APACHE)
or the
[MIT license](https://github.com/ymgyt/syndicationd/blob/main/LICENSE-MIT).

[workflow-ci-badge]: https://img.shields.io/github/actions/workflow/status/ymgyt/syndicationd/ci.yaml?style=for-the-badge&logo=github&label=CI
[workflow-ci-url]: https://github.com/ymgyt/syndicationd/actions/workflows/ci.yaml
[workflow-release-badge]: https://img.shields.io/github/actions/workflow/status/ymgyt/syndicationd/release.yml?style=for-the-badge&logo=github&label=Release
[workflow-release-url]: https://github.com/ymgyt/syndicationd/actions/workflows/release.yml
[workflow-audit-badge]: https://img.shields.io/github/actions/workflow/status/ymgyt/syndicationd/audit.yaml?style=for-the-badge&logo=github&label=Audit
[workflow-audit-url]: https://github.com/ymgyt/syndicationd/actions/workflows/audit.yaml
[coverage-badge]: https://img.shields.io/codecov/c/github/ymgyt/syndicationd?token=W1A93WSPEE&style=for-the-badge&logo=codecov&color=brightgreen
[coverage-url]: https://app.codecov.io/github/ymgyt/syndicationd
