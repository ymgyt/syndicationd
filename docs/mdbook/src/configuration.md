# Configuration

Settings can be configured in the following ways, in order of priority:

* Command line flag
* Environment variables
* Configuration file
* Default value

The location of the configuration file can be specified using `--config` or the
environment variable `SYND_CONFIG_FILE`.

By default, `synd` searches the following locations depending on the platform:

| Platform | Locations |
| ---      | ---       |
| Linux    | `$XDG_CONFIG_HOME/syndicationd/config.toml`<br>`$HOME/.config/syndicationd/config.toml` |
| macOS    | `$HOME/Library/Application Support/syndicationd/config.toml` |
| Windows  | `{FOLDERID_RoamingAppData}/syndicationd/config.toml` |

`synd` does not automatically create configuration files.

To create one, run:

```sh
synd config init > config.toml
```

To inspect the resolved configuration after flags, environment variables,
configuration files, and defaults have been applied, use `config view`.

```sh
synd config view
synd config view -o json
```

## Common Settings

| Flag               | Environment variable     | Configuration file       | Default                | Description                 |
|--------------------|--------------------------|--------------------------|------------------------|-----------------------------|
| `--config`         | `SYND_CONFIG_FILE`       | -                        | see above              | Configuration file path     |
| `--log`            | `SYND_LOG_FILE`          | `[log.path]`             | see `synd config view` | Log file path               |
| `--cache-dir`      | `SYND_CACHE_DIR`         | `[cache.directory]`      | see `synd config view` | Cache directory             |
| `--theme`          | `SYND_THEME`             | `[theme.name]`           | `ferra`                | Theme name                  |
| `--sqlite-db`      | `SYND_SQLITE_DB`         | `[backend.sqlite_db]`    | see `synd config view` | SQLite database path        |
| `--entries-limit`  | `SYND_ENTRIES_LIMIT`     | `[feed.entries_limit]`   | `200`                  | Feed entries to fetch       |
| `--browser`        | `SYND_BROWSER`           | `[feed.browser.command]` | -                      | Command to browse feed      |
| `--browser-args`   | `SYND_BROWSER_ARGS`      | `[feed.browser.args]`    | `[]`                   | Command args to browse feed |

## Theme

The theme can be changed with `--theme`, `SYND_THEME`, or `[theme.name]`.
Run `synd --help` to see the available values.

## Log

The default log file path is based on `ProjectDirs::data_dir()`.
Use `synd config view` to inspect the resolved output destination.

The log directives can be changed with the `SYND_LOG` environment variable.

```sh
SYND_LOG=synd=debug synd
```

## Local Data

The local database path can be set with `--sqlite-db`, `SYND_SQLITE_DB`, or
`[backend.sqlite_db]`.

```sh
synd --sqlite-db ~/.local/share/syndicationd/synd.db
```

## Additional Categories

To add a category, add the following content to the configuration file:

```toml
[categories.rust]
icon = { symbol = "R", color = { rgb = 0xF74C00 } }
aliases = ["rs"]
```
