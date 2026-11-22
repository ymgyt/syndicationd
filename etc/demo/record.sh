#!/usr/bin/env bash
set -o errexit
set -o nounset
set -o pipefail

function build_binary() {
  cargo build --locked --manifest-path "$1" --bin "$2" --message-format=json-render-diagnostics |
    jq -r --arg bin "$2" 'select(.target.name == $bin and .executable != null) | .executable'
}

function send_keys() {
  zellij --session "$session" action send-keys "$@"
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

  local feeds=etc/demo/feeds.json
  "$synd" feed import "$feeds"
  local deadline=$((SECONDS + 90))
  until "$synd" feed export | jq -e --slurpfile expected "$feeds" '
    ([.feeds[] | select(.title != null) | .url] | sort) == ($expected[0].feeds | map(.url) | sort)
  ' >/dev/null; do
    if (( SECONDS >= deadline )); then
      echo 'Timed out fetching demo feeds.' >&2
      "$synd" feed export >&2
      return 1
    fi
    sleep 1
  done

  printf 'session_serialization false\npane_frames false\n' > "$work/zellij.kdl"
  printf 'layout { pane command="synd"; }\n' > "$work/layout.kdl"
  PATH="$(dirname "$synd"):$PATH" alacritty -T 'synd demo' -o 'window.dynamic_title=false' \
    -o 'window.startup_mode="Maximized"' -o 'font.size=19' \
    -e zellij --config "$work/zellij.kdl" --session "$session" \
      --new-session-with-layout "$work/layout.kdl" &
  window_pid=$!
  deadline=$((SECONDS + 20))
  until [[ $(zellij --session "$session" action dump-screen 2>/dev/null) == *'Entry 1/'* ]]; do
    kill -0 "$window_pid"
    if (( SECONDS >= deadline )); then
      echo 'Timed out waiting for the entry list.' >&2
      zellij --session "$session" action dump-screen >&2
      return 1
    fi
    sleep 0.2
  done

  echo 'Select the “synd demo” window in the screen sharing dialog.' >&2
  coproc RECORDER { exec "$wlrec" "$work/demo.gif"; }
  recorder_pid=$! # PID of the wlrec coprocess.
  local status
  IFS= read -r status <&"${RECORDER[0]}" && [[ $status == ready ]] || return 1
  play_demo
  kill -INT "$recorder_pid"
  wait "$recorder_pid"
  recorder_pid=
  mv -- "$work/demo.gif" etc/demo/demo.gif
}

main "$@"
