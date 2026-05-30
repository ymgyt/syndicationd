# Keymap

`synd` key bindings are grouped by keymap layer. A layer is an active UI
context, such as `entries`, `feeds`, or `filter`.

## Custom Key Bindings

Key bindings can be customized in the configuration file under
`[keys.<layer>]`.

```toml
[keys.entries]
keymap = [
  { on = "j", command = "entries.next", desc = "Next entry" },
  { on = ["g", "g"], command = "entries.first", desc = "Go to first entry" },
  { on = "up", command = "no_op" },
]
```

`on` is either a single key string or an array of key strings for multi-key
sequences. The example above binds `g` followed by `g`.

User keymaps are merged onto the default keymaps:

* a binding with the same `on` in the same layer replaces the default binding
* a binding with a new `on` is added to that layer
* `command = "no_op"` disables that key sequence in that layer

`desc` is optional description metadata for the binding.

Commands are validated against the layer. For example, `entries.next` is valid
in the `entries` layer, but not in the `feeds` layer.

Key names use the same notation as the default keymap, including `enter`,
`space`, `tab`, `backtab`, `esc`, arrow keys, `C-c` for Control-C, `S-t` for
Shift-T, and `A-enter` for Alt-Enter.

Configurable layers:

| Layer                                | Context                          |
| ---                                  | ---                              |
| `app`                                | Always active application keys   |
| `global`                             | Always active normal keys        |
| `login`                              | Authentication screen            |
| `tabs`                               | Tab navigation                   |
| `entries`                            | Entries tab                      |
| `feeds`                              | Feeds tab                        |
| `filter`                             | Entry/feed filter controls       |
| `unsubscribe-popup`                  | Feed unsubscribe confirmation    |
| `github-notifications`               | GitHub notifications tab         |
| `github-notification-filter-popup`   | GitHub notification filter popup |

`category-filter` and `search-prompt` are runtime-generated layers. Their
dynamic bindings are managed by the application for now.

## Commands

| Command                                                | Layer                              |
| ---                                                    | ---                                |
| `no_op`                                                | any layer                          |
| `app.quit`                                             | `app`, `global`                    |
| `theme.rotate`                                         | `global`                           |
| `login.authenticate`                                   | `login`                            |
| `login.provider.prev`                                  | `login`                            |
| `login.provider.next`                                  | `login`                            |
| `tabs.prev`                                            | `tabs`                             |
| `tabs.next`                                            | `tabs`                             |
| `entries.prev`                                         | `entries`                          |
| `entries.next`                                         | `entries`                          |
| `entries.first`                                        | `entries`                          |
| `entries.last`                                         | `entries`                          |
| `entries.reload`                                       | `entries`                          |
| `entries.open`                                         | `entries`                          |
| `entries.browse`                                       | `entries`                          |
| `feeds.prev`                                           | `feeds`                            |
| `feeds.next`                                           | `feeds`                            |
| `feeds.first`                                          | `feeds`                            |
| `feeds.last`                                           | `feeds`                            |
| `feeds.subscribe`                                      | `feeds`                            |
| `feeds.edit`                                           | `feeds`                            |
| `feeds.unsubscribe`                                    | `feeds`                            |
| `feeds.refresh-selected`                               | `feeds`                            |
| `feeds.reload`                                         | `feeds`                            |
| `feeds.open`                                           | `feeds`                            |
| `feeds.unsubscribe-popup.prev`                         | `unsubscribe-popup`                |
| `feeds.unsubscribe-popup.next`                         | `unsubscribe-popup`                |
| `feeds.unsubscribe-popup.select`                       | `unsubscribe-popup`                |
| `feeds.unsubscribe-popup.cancel`                       | `unsubscribe-popup`                |
| `filter.requirement.prev`                              | `filter`                           |
| `filter.requirement.next`                              | `filter`                           |
| `filter.category`                                      | `filter`                           |
| `filter.search`                                        | `filter`                           |
| `filter.close`                                         | `filter`                           |
| `github-notifications.prev`                            | `github-notifications`             |
| `github-notifications.next`                            | `github-notifications`             |
| `github-notifications.first`                           | `github-notifications`             |
| `github-notifications.last`                            | `github-notifications`             |
| `github-notifications.open`                            | `github-notifications`             |
| `github-notifications.open-and-done`                   | `github-notifications`             |
| `github-notifications.reload`                          | `github-notifications`             |
| `github-notifications.mark-done`                       | `github-notifications`             |
| `github-notifications.mark-all-done`                   | `github-notifications`             |
| `github-notifications.unsubscribe-thread`              | `github-notifications`             |
| `github-notifications.filter.open`                     | `github-notifications`             |
| `github-notifications.filter.close`                    | `github-notification-filter-popup` |
| `github-notifications.filter.include-unread.toggle`    | `github-notification-filter-popup` |
| `github-notifications.filter.participating.toggle`     | `github-notification-filter-popup` |
| `github-notifications.filter.visibility-public.toggle` | `github-notification-filter-popup` |
| `github-notifications.filter.visibility-private.toggle` | `github-notification-filter-popup` |
| `github-notifications.filter.pr-open.toggle`           | `github-notification-filter-popup` |
| `github-notifications.filter.pr-closed.toggle`         | `github-notification-filter-popup` |
| `github-notifications.filter.pr-merged.toggle`         | `github-notification-filter-popup` |
| `github-notifications.filter.reason-mentioned.toggle`  | `github-notification-filter-popup` |
| `github-notifications.filter.reason-review-requested.toggle` | `github-notification-filter-popup` |

## Default Key Bindings

| Key     | Description                                    |
| ---     | ---                                            |
| `k/j`   | Move up/down                                   |
| `gg`    | Go to first                                    |
| `ge`    | Go to end                                      |
| `Tab`   | Switch tab                                     |
| `Enter` | Open entry/feed with web browser               |
| `Space` | Open entry with text browser (`$SYND_BROWSER`) |
| `a`     | Add feed subscription on the Feeds tab         |
| `e`     | Edit subscribed feed on the Feeds tab          |
| `d`     | Delete subscribed feed on the Feeds tab        |
| `r`     | Reload entries/feeds                           |
| `h/l`   | Change requirement filter                      |
| `c`     | Activate category filter (`Esc` to deactivate) |
| `+`     | Activate all categories on category filter     |
| `-`     | Deactivate all categories on category filter   |
| `/`     | Activate keyword search (`Esc` to deactivate)  |
| `q`     | Quit app                                       |
