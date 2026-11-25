#!/usr/bin/env bash

function build_binary() {
  cargo build --locked --manifest-path "$1" --bin "$2" --message-format=json-render-diagnostics |
    jq -r --arg bin "$2" 'select(.target.name == $bin and .executable != null) | .executable'
}

function fetch_demo_feeds() {
  local synd=$1 feeds=$2 deadline=$((SECONDS + 90))
  "$synd" feed import "$feeds"
  until "$synd" feed export | jq -e --slurpfile expected "$feeds" '
    ([.feeds[] | select(.title != null) | .url] | sort) == ($expected[0].feeds | map(.url) | sort)
  ' >/dev/null; do
    if (( SECONDS >= deadline )); then
      printf 'Timed out fetching demo feeds.\n' >&2
      "$synd" feed export >&2
      return 1
    fi
    sleep 1
  done
}
