#!/usr/bin/env bash

set -o nounset
set -o errexit
set -o pipefail
set -o errtrace

# shellcheck source=etc/demo/common.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"

function wallpaper_path() {
  local state=${XDG_STATE_HOME:-$HOME/.local/state}/cosmic/com.system76.CosmicBackground/v1/wallpapers
  local wallpaper=${SYND_DEMO_WALLPAPER:-}
  if [[ -z $wallpaper && -r $state ]]; then
    # COSMIC stores one (output, Path("...")) tuple per line.
    wallpaper=$(sed -n 's/.*Path(\(".*"\)).*/\1/p' "$state" | jq -rs 'first // empty')
  fi
  if [[ ! -f $wallpaper || ! -r $wallpaper ]]; then
    printf 'Set SYND_DEMO_WALLPAPER to a readable wallpaper image.\n' >&2
    return 1
  fi
  realpath -- "$wallpaper"
}

function start_display() {
  cat > "$work/alacritty.toml" <<'TOML'
[window]
opacity = 0.72
decorations = "None"
padding = { x = 14, y = 10 }
dynamic_padding = true

[font]
normal.family = "JetBrainsMono Nerd Font Mono"
size = 13.5
TOML
  printf 'set -g status off\nset -g default-terminal tmux-256color\nset -g default-shell "%s"\n' \
    "$(command -v bash)" > "$work/tmux.conf"
  cat > "$work/sway.conf" <<'SWAY'
xwayland disable
output HEADLESS-1 mode 1280x640
output HEADLESS-1 bg wallpaper fill
default_border none
gaps inner 0
gaps vertical 20
gaps horizontal 76
exec alacritty --config-file alacritty.toml -e tmux -f tmux.conf new-session -s demo ./synd
SWAY
  (cd "$work" && exec sway -c "$work/sway.conf") > "$work/sway.log" 2>&1 &
  sway_pid=$!
}

function wait_for_entries() {
  local priority=$1 screen deadline=$((SECONDS + 20))
  while kill -0 "$sway_pid" 2>/dev/null && (( SECONDS < deadline )); do
    screen=$(tmux capture-pane -pt demo 2>/dev/null) || screen=
    if [[ $screen =~ Filter[[:blank:]]+"$priority" && $screen == *"Entry 1/"[1-9]* && $screen != *"Fetch timeline"* ]]; then
      return 0
    fi
    sleep 0.1
  done
  cat "$work/sway.log" >&2
  printf 'Timed out waiting for the %s entry list in Alacritty.\n' "$priority" >&2
  return 1
}

function capture() {
  wait_for_entries MAY
  tmux send-keys -t demo l  # Show the latest article from the Must feeds.
  wait_for_entries MST
  export WAYLAND_DISPLAY SWAYSOCK
  WAYLAND_DISPLAY=$(tmux display-message -pt demo '#{WAYLAND_DISPLAY}')
  SWAYSOCK=$(tmux display-message -pt demo '#{SWAYSOCK}')

  grim -l 9 -o HEADLESS-1 "$work/social.png"
  if (( $(wc -c < "$work/social.png") >= 1000000 )); then
    pngquant --strip --output "$staging/social-preview.png" -- "$work/social.png"
  else
    mv -- "$work/social.png" "$staging/social-preview.png"
  fi
  swaymsg -q 'output HEADLESS-1 mode 1440x810; gaps horizontal current set 28; gaps vertical current set 24'
  # Let Alacritty repaint the resized article list.
  sleep 0.4
  grim -l 9 -o HEADLESS-1 "$staging/readme.png"
}

function cleanup() {
  if [[ -n ${sway_pid:-} ]]; then
    tmux kill-server >/dev/null 2>&1 || true
    kill "$sway_pid" 2>/dev/null || true
    wait "$sway_pid" 2>/dev/null || true
  fi
  "$work/synd" daemon shutdown >/dev/null 2>&1 || true
  rm -rf -- "$work"
  if [[ -n $staging ]]; then rm -rf -- "$staging"; fi
}

function main() {
  local demo_dir wallpaper synd
  demo_dir=$(realpath -- "$(dirname -- "${BASH_SOURCE[0]}")")
  wallpaper=$(wallpaper_path)
  synd=$(build_binary Cargo.toml synd)

  work=$(mktemp -d)
  staging=
  sway_pid=
  export XDG_RUNTIME_DIR="$work/run" XDG_STATE_HOME="$work/state" XDG_CACHE_HOME="$work/cache"
  export XDG_CONFIG_HOME="$work/config" XDG_DATA_HOME="$work/data" TMUX_TMPDIR="$work"
  export SYND_CONFIG_FILE="$work/synd.toml" SYND_SQLITE_DB="$work/demo.db" SYND_LOG_FILE="$work/synd.log"
  export SYND_RUNTIME_ROOT="$work/synd-run" SYND_CACHE_DIR="$work/synd-cache" SYND_ENABLE_GH=false
  export WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=1 WLR_RENDERER=pixman LIBGL_ALWAYS_SOFTWARE=1
  unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET SWAYSOCK DBUS_SESSION_BUS_ADDRESS ALACRITTY_SOCKET TMUX TMUX_PANE
  unset ZELLIJ ZELLIJ_PANE_ID ZELLIJ_SESSION_NAME SYND_THEME SYND_GH_PAT NO_COLOR
  trap cleanup EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM
  staging=$(mktemp -d "$demo_dir/.capture.XXXXXXXX")
  mkdir -m 700 "$XDG_RUNTIME_DIR"
  ln -s -- "$wallpaper" "$work/wallpaper"
  ln -s -- "$synd" "$work/synd"
  printf '[theme]\nbackground = "transparent"\n' > "$SYND_CONFIG_FILE"

  fetch_demo_feeds "$synd" "$demo_dir/feeds.json"
  start_display
  capture
  mv -v -- "$staging"/*.png "$demo_dir/"
}

main "$@"
