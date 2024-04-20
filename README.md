<div class="oranda-hide">

# Syndicationd

</div>

[![Crates.io Version][crates-badge]][crates-url]
![License][license-badge]
[![CI][workflow-ci-badge]][workflow-ci-url]
[![Release][workflow-release-badge]][workflow-release-url]

[crates-badge]: https://img.shields.io/crates/v/synd-term?style=for-the-badge&logo=rust
[crates-url]: https://crates.io/crates/synd-term
[license-badge]: https://img.shields.io/badge/license-MIT%2FApache--2.0-blue?style=for-the-badge
[workflow-ci-badge]: https://img.shields.io/github/actions/workflow/status/ymgyt/syndicationd/ci.yaml?style=for-the-badge&logo=github&label=CI
[workflow-ci-url]: https://github.com/ymgyt/syndicationd/actions/workflows/ci.yaml
[workflow-release-badge]: https://img.shields.io/github/actions/workflow/status/ymgyt/syndicationd/release.yml?style=for-the-badge&logo=github&label=Release
[workflow-release-url]: https://github.com/ymgyt/syndicationd/actions/workflows/release.yml
[website-badge]: https://img.shields.io/badge/website-blue?style=for-the-badge
[website]: https://docs.syndicationd.ymgyt.io/synd-term/

![Demo](./assets/demo.gif)

Syndicationd(`synd`) is a TUI feed viewer, based on [feed-rs](https://github.com/feed-rs/feed-rs) and [ratatui](https://github.com/ratatui-org/ratatui).


## Features

* Subscribe feeds(RSS1, RSS2, Atom, JSON) and browse latest entries 
* Open the entry in a browser
* Filter entries by categories and [requirement](#requirement)

## Install

### nix

```sh
nix profile install github:ymgyt/syndicationd
```

### arch linux

```sh
pacman -S syndicationd
```

### brew

```sh
brew tap ymgyt/syndicationd
brew install synd
# or
brew install ymgyt/homebrew-syndicationd/synd
```

### shell

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ymgyt/syndicationd/releases/download/synd-term-v0.2.2/synd-term-installer.sh | sh
```

### cargo

```sh
cargo install synd-term --locked
```

### npm

```sh
npm install @syndicationd/synd-term
```

### powershell

```sh
powershell -c "irm https://github.com/ymgyt/syndicationd/releases/download/synd-term-v0.2.2/synd-term-installer.ps1 | iex"
```

## Usage

`synd` will start the TUI application.

<details>
<summary>Click to show a complete list of options</summary>

```console
Usage: synd [OPTIONS] [COMMAND]

Commands:
  clear   Clear cache, log
  check   Check application conditions
  export  Export subscribed feeds
  help    Print this message or the help of the given subcommand(s)

Options:
      --endpoint <ENDPOINT>                synd_api endpoint [env: SYND_ENDPOINT=] [default: https://api.syndicationd.ymgyt.io:6100]
      --log <LOG>                          Log file path [env: SYND_LOG=] [default: " /home/ymgyt/.local/share/synd/synd.log"]
      --theme <PALETTE>                    Color palette [env: SYND_THEME=] [default: slate] [possible values: slate, gray, zinc, neutral, stone, red, orange, amber,
                                           yellow, lime, green, emerald, teal, cyan, sky, blue, indigo, violet, purple, fuchsia, pink]
      --timeout <TIMEOUT>                  Client timeout [default: 30s]
      --categories <CATEGORIES TOML PATH>  categories.toml path
  -h, --help                               Print help
  -V, --version                            Print version
```

</details>

### Authentication

syndicationd maintains state (such as subscribed feeds) on the backend, and therefore requires authentication to make requests.  
Currently, GitHub and Google are supported as authorize server/id provider. The only scope syndicationd requires is [`user:email`](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps)(Github) or [`email`](https://developers.google.com/identity/gsi/web/guides/devices#obtain_a_user_code_and_verification_url)(Google) to read the user's email. the user's email is used only as an identifier after being hashed and never stored.

### Keymap

<details>
<summary>Click to show a keymap table</summary>

| Key     | Description                                   |
| ---     | ---                                           |
| `k/j`   | Move up/down                                  |
| `gg`    | Go to first                                   |
| `ge`    | Go to end                                     |
| `Tab`   | Switch Tab                                    |
| `Enter` | Open entry/feed                               |
| `a`     | Add feed subscription(on Feeds Tab)           |
| `e`     | Edit subscribed feed(on Feeds Tab)            |
| `d`     | Delete subscribed feed(on Feeds Tab)          |
| `r`     | Reload entries/feeds                          |
| `h/l`   | Change requirement filiter                    |
| `c`     | Activate category filiter(Esc to deactivate)  |
| `+`     | Activate all category(on Category filter)     |
| `-`     | Deactivate all category(on Category filter)   |
| `q`     | Quit app                                      |

</details>

for more details, refer to [`keymap/default.rs`](https://github.com/ymgyt/syndicationd/blob/main/crates/synd_term/src/keymap/default.rs)

### Subscribe feed

To subscribe a feed, type "Tab" to move to Feeds tab and then press "a".  
`synd` uses [edit](https://docs.rs/edit/latest/edit/) to launch the user's editor(like a git commit).  
The feed to subscribe to should be entered in the format:  
`Requirement` `Category` `URL`  

When you close the editor, the subscription request is sent to the API.

#### Requirement

`Requirement` indicates the importance of the feed.  
This uses an analogy to [RFC2119](https://datatracker.ietf.org/doc/html/rfc2119) and can take one of the following values:

* `MUST`: Most important, must be read.
* `SHOULD`: Next in importance, should be read unless there is a special reason not to.
* `MAY`: Lowest importance, may be read.

#### Category

`Category` represents the category of the feed. You can specify any value as a category. The values that `synd` recognizes as categories are defined in [`categories.toml`](./categories.toml). You can override the default values with the `--categories` flag.


### Edit subscribed feed

To change the requirement or category of a feed you have already subscribed to, select the target feed in the Feeds tab and then press "e".

### Unsubscribe feed

To unsubscribe from a feed, select the target feed and press "d".

### Filter feeds/entries

Feeds and entries can be filtered as follows.

#### By requirement

To filter bases on the specified requirement, press "h/l(Left/Right)".  
If you set the filter to `MUST`, only those marked as MUST will be displayed. Setting it to SHOULD will display feeds and entries marked as MUST and SHOULD. If set to MAY, all feeds and entries will be displayed.

#### By categories

To filter bases on categories, presess "c". This will display a label with keys to control the activation/deactivation of each category, allowing you to toggle the visibility of categories.  
Pressing "-" will deactivate all categories, and pressing "+" will activate all categories.  

You can exit the filter category mode by pressing the "Esc" key.  
The icons for categories can be specified in `categories.toml`


### Export subscribed feeds

To export subscribed feeds, execute the `synd export` command.  
You can check the JSON schema of the data to be exported with `synd export --print-schema`

<details>
<summary>Click to show a export json schema</summary>

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Export",
  "type": "object",
  "required": [
    "feeds"
  ],
  "properties": {
    "feeds": {
      "type": "array",
      "items": {
        "$ref": "#/definitions/ExportedFeed"
      }
    }
  },
  "definitions": {
    "ExportedFeed": {
      "type": "object",
      "required": [
        "url"
      ],
      "properties": {
        "title": {
          "type": [
            "string",
            "null"
          ]
        },
        "url": {
          "type": "string"
        }
      }
    }
  }
}
```
</details>

### Log file

The log file path is based on [`ProjectDirs::data_dir()`](https://docs.rs/directories/latest/directories/struct.ProjectDirs.html#method.data_dir).  
Please refer to the `--log` flag in `synd --help` for the default output destination.  

You can modify the [log directives](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives) using the environment variable `SYND_LOG`. (for example, `SYND_LOG=synd=debug`)

### Theme

The theme can be changed using the `--theme` flag. Please refer to the help for the values that can be specified.

### Backend api

By default, `synd` use `https://api.syndicationd.ymgyt.io` as the [backend api](./crates/synd_api)([hosted on my home Raspberry Pi](https://github.com/ymgyt/mynix/blob/main/homeserver/modules/syndicationd/default.nix)).  
To change the endpoint, specify the `--endpoint` flag

The hosted api is instrumented with OpenTelemetry. Basic signals(traces,metrics,logs) are published on the [Grafana dashboard](https://ymgyt.grafana.net/public-dashboards/863ebddd82c44ddd9a28a68eaac848ff?orgId=1&refresh=1h&from=now-1h&to=now)


### Clear cache and logs

Authentication credentials are cached. to remove them, execute `synd clear`.

### Check application status

`synd check [--format (human|json)]` return current application status.

```console
synd check --format json | from json
╭───────┬─────────────────────────────────────────╮
│       │ ╭─────────────┬────────────────────╮    │
│ api   │ │ description │ health of synd-api │    │
│       │ │ status      │ Pass               │    │
│       │ │ version     │ 0.1.9              │    │
│       │ ╰─────────────┴────────────────────╯    │
│ cache │ /home/ferris/.cache/synd                │
│ log   │ /home/ferris/.local/share/synd/synd.log │
╰───────┴─────────────────────────────────────────╯
```

## License

This project is available under the terms of either the [Apache 2.0 license](./LICENSE-APACHE) or the [MIT license](./LICENSE-MIT).

## Feed tips

Some tips about feed that I know.

* Add [`openrss.org/`](https://openrss.org/) to the beginning of the URL to get its RSS feed. for example, for `https://example.ymgyt.io`, it would be `https://openrss.org/example.ymgyt.io`

* You can retrieve various updates as feeds on GitHub.
  * To obtain releases of a repository, specify `releases.atom`. for example, to obtain releases of syndicationd, specify `https://github.com/ymgyt/syndicationd/releases.atom`
  * For tags, it's `https://github.com/ymgyt/syndicationd/tag.atom` 

* Adding `.rss` to the end of a Reddit URL allows you to retrieve the feed. for example, for `https://www.reddit.com/r/HelixEditor/`, it would be `https://www.reddit.com/r/HelixEditor.rss`

If you're looking for feeds, here are my recommendations.

### For Rust users

<details>
<summary>Click to show a table</summary>

| Feed | URL |
| ---  | --- |
| [This Week in Rust](https://this-week-in-rust.org/) | `https://this-week-in-rust.org/atom.xml`|
| [Rust Blog](https://without.boats/index.xml) | `https://blog.rust-lang.org/feed.xml` |
| [Inside Rust Blog](https://blog.rust-lang.org/inside-rust/) | `https://blog.rust-lang.org/inside-rust/feed.xml` |
| [RustSec Advisories](https://rustsec.org/) | `https://rustsec.org/feed.xml` |
| [seanmonstar](https://seanmonstar.com/) | `https://seanmonstar.com/rss` |
| [Mara's Blog](https://blog.m-ou.se/) | `https://blog.m-ou.se/index.xml` |
| [Without boats, dreams dry up](https://without.boats/) | `https://without.boats/index.xml` |
| [fasterthanli.me](https://fasterthanli.me/) | `https://fasterthanli.me/index.xml` |
| [Orhun's Blog](https://blog.orhun.dev/) | `https://blog.orhun.dev/rss.xml` |
| [axo blog](https://blog.axo.dev/) | `https://blog.axo.dev/rss.xml` |
| [Kbzol's blog](https://kobzol.github.io/) | `https://kobzol.github.io/feed.xml` |
| [baby steps](https://smallcultfollowing.com/babysteps/) | `https://smallcultfollowing.com/babysteps/` |
| [COCl2's blog home](https://blog.cocl2.com/) | `https://blog.cocl2.com/index.xml` |

</details>

### For Nix users

<details>
<summary>Click to show a table</summary>

| Feed | URL |
| ---  | --- |
| [This Cute World](https://thiscute.world/en/) | `https://thiscute.world/en/index.xml` |
| [Determinate Systems](https://determinate.systems/) | `https://determinate.systems/rss.xml` |
</details>


### For Observability 

<details>
<summary>Click to show a table</summary>

| Feed | URL |
| ---  | --- |
| [observability news](https://buttondown.email/o11y.news) | `https://buttondown.email/o11y.news/rss` |
| [Opentelemetry blog](https://opentelemetry.io/blog/2024/) | `https://opentelemetry.io/blog/2024/index.xml` |
| [eBPF Blog](https://ebpf.io/blog/) | `https://ebpf.io/blog/rss.xml` |

</details>

### For Kubernetes users

<details>
<summary>Click to show a table</summary>

| Feed | URL |
| ---  | --- |
| [Kubernetes Blog](https://kubernetes.io/) | ` https://kubernetes.io/feed.xml` |
| [Kubernetes Official CVE](https://kubernetes.io/docs/reference/issues-security/official-cve-feed/) | `https://kubernetes.io/docs/reference/issues-security/official-cve-feed/feed.xml` |
| [CNCF](https://www.cncf.io/) | `https://www.cncf.io/feed/` |
</details>

### Misc

<details>
<summary>Click to show a table</summary>

| Feed | URL |
| ---  | --- |
| [Terminal Trove](https://blog.cocl2.com/index.xml) | `https://terminaltrove.com/blog.xml` |

</details>
