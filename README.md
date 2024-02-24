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

### brew

```sh
brew tap ymgyt/syndicationd
brew install synd
# or
brew install ymgyt/homebrew-syndicationd/synd
```

### shell

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ymgyt/syndicationd/releases/download/synd-term-v0.1.4/synd-term-installer.sh | sh
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
powershell -c "irm https://github.com/ymgyt/syndicationd/releases/download/synd-term-v0.1.4/synd-term-installer.ps1 | iex"
```

## Usage

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

## License

This project is available under the terms of either the [Apache 2.0 license](./LICENSE-APACHE) or the [MIT license](./LICENSE-MIT).
