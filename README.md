<div class="oranda-hide">

# Syndicationd

</div>

[![CI](https://github.com/ymgyt/syndicationd/actions/workflows/ci.yaml/badge.svg)](https://github.com/ymgyt/syndicationd/actions/workflows/ci.yaml)
[![Release](https://github.com/ymgyt/syndicationd/actions/workflows/release.yml/badge.svg)](https://github.com/ymgyt/syndicationd/actions/workflows/release.yml)

![Demo](./assets/demo.gif)

Syndicationd(`synd`) is a TUI feed viewer, based on [feed-rs](https://github.com/feed-rs/feed-rs) and [ratatui](https://github.com/ratatui-org/ratatui).

[Website](https://docs.syndicationd.ymgyt.io/synd-term/)

## Features

* Subscribe feeds(RSS1, RSS2, Atom,...) and browse latest entries 
* Open the entry in a browser


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
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ymgyt/syndicationd/releases/download/synd-term-v0.1.10/synd-term-installer.sh | sh
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
powershell -c "irm https://github.com/ymgyt/syndicationd/releases/download/synd-term-v0.1.10/synd-term-installer.ps1 | iex"
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
      --endpoint <ENDPOINT>  synd_api endpoint [env: SYND_ENDPOINT=] [default:
                             https://api.syndicationd.ymgyt.io:6100]
      --log <LOG>            Log file path [env: SYND_LOG=] [default:
                             /home/ymgyt/.local/share/synd/synd.log]
      --theme <PALETTE>      Color palette [env: SYND_THEME=] [default: slate] [possible values: slate,
                             gray, zinc, neutral, stone, red, orange, amber, yellow, lime, green,
                             emerald, teal, cyan, sky, blue, indigo, violet, purple, fuchsia, pink]
      --timeout <TIMEOUT>    Client timeout [default: 30s]
  -h, --help                 Print help
  -V, --version              Print version
 ```

</details>

### Authentication

syndicationd maintains state (such as subscribed feeds) on the backend, and therefore requires authentication to make requests.  
Currently, GitHub and Google are supported. The only scope syndicationd requires is [`user:email`](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps)(Github) or [`email`](https://developers.google.com/identity/gsi/web/guides/devices#obtain_a_user_code_and_verification_url)(Google) to read the user's email. the user's email is used only as an identifier after being hashed and never stored.

### Keymap

<details>
<summary>Click to show a keymap table</summary>

| Key     | Description                          |
| ---     | ---                                  |
| `k/j`   | Move up/down                         |
| `gg`    | Go to first                          |
| `ge`    | Go to end                            |
| `Tab`   | Switch Tab                           |
| `Enter` | Open entry/feed                      |
| `a`     | Add feed subscription(on Feeds Tab)  |
| `d`     | Delete subscribed feed(on Feeds Tab) |
| `r`     | Reload entries/feeds                 |
| `q`     | Quit app                             |

</details>

for more details, refer to [`keymap/default.rs`](https://github.com/ymgyt/syndicationd/blob/main/crates/synd_term/src/keymap/default.rs)


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

By default, use `https://api.syndicationd.ymgyt.io` as the [backend api](./crates/synd_api)([hosted on my home Raspberry Pi](https://github.com/ymgyt/mynix/blob/main/homeserver/modules/syndicationd/default.nix)).  
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
│       │ │ version     │ 0.1.5              │    │
│       │ ╰─────────────┴────────────────────╯    │
│ cache │ /home/ferris/.cache/synd                │
│ log   │ /home/ferris/.local/share/synd/synd.log │
╰───────┴─────────────────────────────────────────╯
```

## License

This project is available under the terms of either the [Apache 2.0 license](./LICENSE-APACHE) or the [MIT license](./LICENSE-MIT).
