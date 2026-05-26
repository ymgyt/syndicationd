# GitHub Notifications

GitHub notification support is optional. It is separate from the core RSS/Atom
feed workflow and is disabled by default.

## Enable

Enable the feature with the command line flag or environment variable:

```sh
synd --enable-github-notification true --github-pat <PAT>
SYND_ENABLE_GH=true SYND_GH_PAT=<PAT> synd
```

The equivalent configuration file entries are:

```toml
[github]
enable = true
pat = "<PAT>"
```

## Keymap

| Key | Description                            |
| --- | ---                                    |
| `d` | Mark the selected notification as done |
| `D` | Mark all notifications as done         |
| `u` | Unsubscribe from the selected thread   |
| `f` | Open the notification filter           |

For the complete keymap, see
[`keymap/default.rs`](https://github.com/ymgyt/syndicationd/blob/main/crates/synd_term/src/keymap/default.rs).

## Token Scopes

### Classic Token

The `repo` scope is required.

### Fine-Grained Token

"Metadata" repository permissions (read) and "Notifications" user permissions
(read) are required.

Marking a notification as done is not supported by GitHub's fine-grained token
API. Use a classic token if you need that operation.
