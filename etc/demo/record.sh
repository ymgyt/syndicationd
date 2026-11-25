#!/usr/bin/env bash
set -o errexit
set -o nounset
set -o pipefail

readonly WIDTH=1280 HEIGHT=640

# shellcheck source=etc/demo/common.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"

function send_keys() {
  zellij --session "$session" action send-keys --pane-id "$pane" "$@"
}

function size_demo_pane() {
  # Alacritty reports cell dimensions in physical pixels after applying display scaling.
  local cell_width cell_height
  read -r cell_width cell_height < <(awk '/Cell size:/ { w=$(NF-2); h=$NF } END { print w, h }' "$work/alacritty.log")
  if ! [[ $cell_width =~ ^[1-9][0-9]*$ && $cell_height =~ ^[1-9][0-9]*$ ]]; then
    echo 'Could not determine the Alacritty cell size.' >&2
    return 1
  fi

  # Leave one terminal cell around the TUI inside the capture rectangle.
  local columns=$((WIDTH / cell_width - 2)) rows=$((HEIGHT / cell_height - 2))
  local panes
  panes=$(zellij --session "$session" action list-panes --all --json)
  if ! jq -e --argjson w "$cell_width" --argjson h "$cell_height" \
      --argjson width "$WIDTH" --argjson height "$HEIGHT" \
      'any(.[]; (.is_plugin or .is_floating | not) and .pane_columns * $w >= $width and .pane_rows * $h >= $height)' \
      <<< "$panes" >/dev/null; then
    echo "The demo window must fit a ${WIDTH}×${HEIGHT} recording." >&2
    return 1
  fi
  pane=$(jq -er '.[] | select(.is_floating and .title == "synd") | "terminal_\(.id)"' <<< "$panes")
  zellij --session "$session" action change-floating-pane-coordinates --pane-id "$pane" \
    --x 1 --y 1 --width "$columns" --height "$rows"
  pane_position="$cell_width,$cell_height"
  pane_crop="${pane_position}+$((columns * cell_width))x$((rows * cell_height))"
}

function render_gif() {
  local recording=$1 output=$2 crop=$3 position=$4
  gifsicle --crop "$crop" --position "$position" --logical-screen "${WIDTH}x${HEIGHT}" \
    --background '#2b292d' --optimize=2 "$recording" -o "$output"
}

function play_demo() {
  sleep 1
  send_keys j
  sleep 0.5
  send_keys l  # Show Must entries.
  sleep 1.5
  send_keys h Tab
  sleep 2
  send_keys j
  sleep 1
  send_keys Tab
  sleep 1
  send_keys /
  sleep 0.5
  local key
  for key in r u s t; do
    send_keys "$key"
    sleep 0.2
  done
  sleep 2
  send_keys Esc j
  sleep 1
  send_keys T  # Show the same search results in two more themes.
  sleep 2
  send_keys T
  sleep 8
}

function cleanup() {
  if [[ -n ${recorder_pid:-} ]]; then kill "$recorder_pid" 2>/dev/null || true; fi
  zellij kill-session "$session" >/dev/null 2>&1 || true
  if [[ -n ${window_pid:-} ]]; then kill "$window_pid" 2>/dev/null || true; fi
  "$synd" daemon shutdown >/dev/null 2>&1 || true
  rm -rf -- "$work"
}

function main() {
  synd=$(build_binary Cargo.toml synd)
  local wlrec
  wlrec=$(build_binary "$1/Cargo.toml" wlrec)

  work=$(mktemp -d)
  export SYND_CONFIG_FILE="$work/config.toml" SYND_SQLITE_DB="$work/demo.db"
  export SYND_RUNTIME_ROOT="$work/run" SYND_CACHE_DIR="$work/cache" SYND_LOG_FILE="$work/synd.log"
  export SYND_ENABLE_GH=false
  session="synd-demo-${work##*.}"
  unset SYND_THEME SYND_GH_PAT NO_COLOR ZELLIJ ZELLIJ_PANE_ID ZELLIJ_SESSION_NAME
  touch "$SYND_CONFIG_FILE"
  trap cleanup EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM

  fetch_demo_feeds "$synd" etc/demo/feeds.json

  printf 'session_serialization false\npane_frames false\nshow_release_notes false\n' > "$work/zellij.kdl"
  PATH="$(dirname "$synd"):$PATH" alacritty -v -T 'synd demo' -o 'window.dynamic_title=false' \
    -o 'window.startup_mode="Maximized"' -o 'window.padding={x=0,y=0}' -o 'window.dynamic_padding=false' -o 'font.size=12.5' \
    -e zellij --config "$work/zellij.kdl" --session "$session" \
      --new-session-with-layout "$PWD/etc/demo/layout.kdl" > "$work/alacritty.log" 2>&1 &
  window_pid=$!
  local deadline=$((SECONDS + 20))
  until [[ $(zellij --session "$session" action dump-screen 2>/dev/null) == *'Entry 1/'* ]]; do
    kill -0 "$window_pid"
    if (( SECONDS >= deadline )); then
      echo 'Timed out waiting for the entry list.' >&2
      zellij --session "$session" action dump-screen >&2
      return 1
    fi
    sleep 0.2
  done
  size_demo_pane

  echo 'Select the “synd demo” window in the screen sharing dialog.' >&2
  coproc RECORDER { exec "$wlrec" "$work/window.gif"; }
  recorder_pid=$! # PID of the wlrec coprocess.
  local status
  IFS= read -r status <&"${RECORDER[0]}" && [[ $status == ready ]] || return 1
  play_demo
  kill -INT "$recorder_pid"
  wait "$recorder_pid"
  recorder_pid=
  render_gif "$work/window.gif" "$work/demo.gif" "$pane_crop" "$pane_position"
  mv -- "$work/demo.gif" etc/demo/demo.gif
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
  main "$@"
fi
