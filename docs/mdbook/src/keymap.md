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

In a list, the status line shows one key binding for each main action. For
example, Entries shows `Ent:󰏌` to open the selected entry and `Sp:󰏋` to
read the article from its website in the built-in reader. To open an article
with your configured text browser, bind `entries.browse` in `[keys.entries]`.
Within each layer, configured keys take precedence over built-in aliases in
the guide. A binding's `desc`, when set, replaces the action icon.

While a key sequence is pending, the status line shows its continuations.
Search, category filters, and popups show their own controls. These hints
use the binding description when available and otherwise show the command name.

Commands are validated against the layer. For example, `entries.next` is valid
in the `entries` layer, but not in the `feeds` layer.

Key names use the same notation as the default keymap, including `enter`,
`space`, `tab`, `backtab`, `esc`, arrow keys, `C-c` for Control-C, `S-t` for
Shift-T, and `A-enter` for Alt-Enter.

Use `[keys.list]` to bind a movement key for entries, feeds, and GitHub
notifications together. A binding in an individual tab takes precedence.

Configurable layers:

| Layer                                | Context                          |
| ---                                  | ---                              |
| `app`                                | Always active application keys   |
| `global`                             | Always active normal keys        |
| `login`                              | Authentication screen            |
| `tabs`                               | Tab navigation                   |
| `list`                               | Navigation in the active list    |
| `entries`                            | Entries tab                      |
| `reader`                             | Built-in article reader          |
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
| `app.redraw`                                           | `app`                              |
| `theme.rotate`                                         | `global`                           |
| `login.authenticate`                                   | `login`                            |
| `login.provider.prev`                                  | `login`                            |
| `login.provider.next`                                  | `login`                            |
| `tabs.prev`                                            | `tabs`                             |
| `tabs.next`                                            | `tabs`                             |
| `list.prev`, `list.next`, `list.first`, `list.last`     | `list`, `entries`, `feeds`, `github-notifications` |
| `entries.prev`                                         | `entries`                          |
| `entries.next`                                         | `entries`                          |
| `entries.first`                                        | `entries`                          |
| `entries.last`                                         | `entries`                          |
| `timeline.refresh`                                     | `entries`                          |
| `entries.open`                                         | `entries`                          |
| `entries.read`                                         | `entries`                          |
| `entries.browse`                                       | `entries`                          |
| `reader.close`                                         | `reader`                           |
| `reader.line-up`, `reader.line-down`                   | `reader`                           |
| `reader.half-page-up`, `reader.half-page-down`         | `reader`                           |
| `reader.page-up`, `reader.page-down`                   | `reader`                           |
| `feeds.prev`                                           | `feeds`                            |
| `feeds.next`                                           | `feeds`                            |
| `feeds.first`                                          | `feeds`                            |
| `feeds.last`                                           | `feeds`                            |
| `feeds.subscribe`                                      | `feeds`                            |
| `feeds.edit`                                           | `feeds`                            |
| `feeds.unsubscribe`                                    | `feeds`                            |
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

| Layer               | Key                    | Action                                                |
| ---                 | ---                    | ---                                                   |
| `app`               | `Ctrl+C`               | Quit                                                  |
| `app`               | `R`                    | Redraw the screen                                     |
| `global`            | `q`                    | Quit                                                  |
| `global`            | `T`                    | Cycle through themes                                  |
| `tabs`              | `Tab` / `Shift+Tab`    | Switch to the next / previous tab                     |
| `list`              | `k` or `↑`             | Move up                                               |
| `list`              | `j` or `↓`             | Move down                                             |
| `list`              | `gg` / `ge`            | Jump to the first / last item                         |
| `entries`           | `Enter`                | Open the entry in your web browser                    |
| `entries`           | `Space`                | Open the article URL in the built-in reader           |
| `reader`            | `j` / `↓`, `k` / `↑`    | Scroll down or up one line                            |
| `reader`            | `z j` / `z k`          | Scroll down or up half a page                         |
| `reader`            | `Space` / `PageDown`, `PageUp` | Scroll down or up one page                     |
| `reader`            | `Esc` / `q`            | Return to Entries                                     |
| `entries`           | `r`                    | Refresh the timeline                                  |
| `feeds`             | `a` / `e`              | Add / edit a subscription                             |
| `feeds`             | `d`                    | Open the unsubscribe confirmation                     |
| `feeds`             | `Enter`                | Open the feed's website in your web browser           |
| `feeds`             | `r`                    | Reload the subscription list                          |
| `filter`            | `h` / `l` or `←` / `→` | Change the priority filter                            |
| `filter`            | `c`                    | Open the category filter                              |
| `filter`            | `/`                    | Search by keyword                                     |
| `filter`            | `Esc`                  | Leave the category filter or search prompt            |
| `category-filter`   | Category shortcut      | Toggle the corresponding category                     |
| `category-filter`   | `+` / `-`              | Enable / disable all categories                       |
| `search-prompt`     | `Backspace`            | Delete the previous character                         |
| `unsubscribe-popup` | `h` / `l` or `←` / `→` | Change the selected choice                            |
| `unsubscribe-popup` | `Enter` / `Esc`        | Confirm the selected choice / cancel                  |
| `login`             | `k` / `j` or `↑` / `↓` | Select the previous / next provider                   |
| `login`             | `Enter`                | Authenticate with the selected provider               |

See the [built-in keymap](https://github.com/ymgyt/syndicationd/blob/main/crates/synd_term/src/keymap/default.rs) for all static bindings.
