# Keymap

A key binding assigns a command to a key or a sequence of keys. A **layer**
is a group of bindings. In the Entries tab, `j` moves to the next item
using the `list` layer, and `Space` opens the reader using the `entries` layer.

| Screen or prompt | Active layers, highest priority first |
| --- | --- |
| Entries tab | `filter` → `entries` → `list` → `tabs` → `global` → `app` |
| Feeds tab | `filter` → `feeds` → `list` → `tabs` → `global` → `app` |
| Typing a search | `search-prompt` just before `filter` in the current tab |
| Choosing categories | `category-filter` just before `filter` in the current tab |
| Article reader | `reader` → `global` → `app` |
| Unsubscribe confirmation | `unsubscribe-popup` → `filter` → `tabs` → `global` → `app` |
| Login screen | `login` → `global` → `app` |

When two layers handle the same key, the one on the left takes priority.
In the Entries list, `q` quits (`global`). In the article reader, `q` closes
the article (`reader`).

## Custom Key Bindings

To read the selected article with `o` in the Entries tab, add this to your
[configuration file](./configuration.md).

```toml
[keys.entries]
keymap = [
  { on = "o", command = "entries.read" },
]
```

Restart `synd` and select an entry. Both `o` and `Space` now open the reader.
Reusing a key in the same layer replaces its default binding. To disable
`Space`, add `{ on = "space", command = "no_op" }` to the same array.

To bind another action, put its layer name after `keys.` and its command name
in `command`. You can find both under [Default Key Bindings](#default-key-bindings).
An optional `desc` supplies the label when the binding appears in the status line.

## Default Key Bindings

Use the key names below in `on`. For a sequence such as `g g`, write
`on = ["g", "g"]`. The prefixes `C-`, `S-`, and `A-` stand for Ctrl,
Shift, and Alt.

### Entries and Feeds

Use `[keys.list]` for movement in both tabs. For a movement key specific to
one tab, use the same `list` command in `[keys.entries]` or `[keys.feeds]`.
These layers also accept tab-specific names such as `entries.next` and
`feeds.first`.

| Layer | Action | Default key | Command |
| --- | --- | --- | --- |
| `list` | Move to the next item | `j` or `down` | `list.next` |
| `list` | Move to the previous item | `k` or `up` | `list.prev` |
| `list` | Jump to the first item | `g g` | `list.first` |
| `list` | Jump to the last item | `g e` | `list.last` |
| `entries` | Read the article in the built-in reader | `space` | `entries.read` |
| `entries` | Open the article in your web browser | `enter` | `entries.open` |
| `entries` | Open the article in your configured text browser | Unbound | `entries.browse` |
| `entries` | Refresh the timeline | `r` | `timeline.refresh` |
| `feeds` | Add a subscription | `a` | `feeds.subscribe` |
| `feeds` | Edit the selected subscription | `e` | `feeds.edit` |
| `feeds` | Open the unsubscribe dialog | `d` | `feeds.unsubscribe` |
| `feeds` | Open the feed's website in your web browser | `enter` | `feeds.open` |
| `feeds` | Reload the subscription list | `r` | `feeds.reload` |

### Article reader

| Action | Default key | Command in `[keys.reader]` |
| --- | --- | --- |
| Scroll down one line | `j` or `down` | `reader.line-down` |
| Scroll up one line | `k` or `up` | `reader.line-up` |
| Scroll down half a page | `z j` | `reader.half-page-down` |
| Scroll up half a page | `z k` | `reader.half-page-up` |
| Scroll down one page | `space` or `pagedown` | `reader.page-down` |
| Scroll up one page | `pageup` | `reader.page-up` |
| Return to Entries | `esc` or `q` | `reader.close` |

### Filters

| Action | Default key | Command in `[keys.filter]` |
| --- | --- | --- |
| Search by keyword | `/` | `filter.search` |
| Choose categories | `c` | `filter.category` |
| Select the previous feed priority | `h` or `left` | `filter.requirement.prev` |
| Select the next feed priority | `l` or `right` | `filter.requirement.next` |
| Leave search or category selection | `esc` | `filter.close` |

`synd` assigns the keys for `search-prompt` and `category-filter`. While
searching, use `Backspace` to delete the previous character. While choosing
categories, press the shortcut beside a category to toggle it, `+` to select
all categories, or `-` to clear the selection.

### Unsubscribe confirmation

The dialog blocks the `feeds` and `list` bindings while you choose whether
to unsubscribe.

| Action | Default key | Command in `[keys.unsubscribe-popup]` |
| --- | --- | --- |
| Select the previous choice | `h` or `left` | `feeds.unsubscribe-popup.prev` |
| Select the next choice | `l` or `right` | `feeds.unsubscribe-popup.next` |
| Confirm the selected choice | `enter` | `feeds.unsubscribe-popup.select` |
| Cancel | `esc` | `feeds.unsubscribe-popup.cancel` |

### Application, tabs, and login

| Layer | Action | Default key | Command |
| --- | --- | --- | --- |
| `app` | Quit | `C-c` | `app.quit` |
| `app` | Redraw the screen | `S-r` | `app.redraw` |
| `global` | Quit | `q` | `app.quit` |
| `global` | Cycle through themes | `S-t` | `theme.rotate` |
| `tabs` | Switch to the next tab | `tab` | `tabs.next` |
| `tabs` | Switch to the previous tab | `backtab` (Shift+Tab) | `tabs.prev` |
| `login` | Select the next provider | `j` or `down` | `login.provider.next` |
| `login` | Select the previous provider | `k` or `up` | `login.provider.prev` |
| `login` | Log in with the selected provider | `enter` | `login.authenticate` |
