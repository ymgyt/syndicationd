#!/usr/bin/env nu

use std/log [info]

const repo_root = path self ../..
const cliff_config = path self ../../cliff.toml
const changelog = path self ../../CHANGELOG.md
const original_changelog = path self ../../docs/CHANGELOG.org.md
const tag_pattern = '^(synd-term-v[0-9].*|v[0-9].*)$'
const rc_tag_pattern = '.*-rc\..*'
const release_range = 'synd-term-v0.3.2..HEAD'

def main [
  spec: string # unreleased | version label | major | minor | patch
] {
  if "GITHUB_TOKEN" not-in ($env | columns) {
    error make { msg: "GITHUB_TOKEN is required to generate the changelog" }
  }

  let target = match $spec {
    "unreleased" => null
    "major" | "minor" | "patch" => { bump_label $spec }
    _ => { normalize_label $spec }
  }
  let release_args = if $target == null {
    []
  } else {
    ["--tag", $target]
  }
  let stable_args = if ($target != null) and (not (is_prerelease $target)) {
    ["--ignore-tags", $rc_tag_pattern]
  } else {
    []
  }

  let cliff_cmd = [
    "git", "cliff",
    "--config", $cliff_config,
    "--repository", $repo_root,
    "--tag-pattern", $tag_pattern,
    "--github-repo", "ymgyt/syndicationd",
    "--strip", "footer",
    "--output", $changelog,
  ]
  | append $release_args
  | append $stable_args
  | append $release_range

  info $"($cliff_cmd | str join ' ')"

  with-env {
    RUST_LOG: "git_cliff_core::remote=info,git_cliff_core=debug",
  } {
    cd $repo_root
    run-external ...$cliff_cmd
  }

  open --raw $original_changelog | save $changelog --append
}

def bump_label [level: string] {
  let cliff_cmd = [
    "git", "cliff",
    "--config", $cliff_config,
    "--repository", $repo_root,
    "--tag-pattern", $tag_pattern,
    "--ignore-tags", $rc_tag_pattern,
    "--offline",
    "--bump", $level,
    "--bumped-version",
  ]

  info $"($cliff_cmd | str join ' ')"
  normalize_label (run-external ...$cliff_cmd | str trim)
}

def normalize_label [label: string] {
  let label = (
    $label
    | str trim
    | str replace --regex '^synd-term-' ''
    | str trim --left --char 'v'
  )

  if not ($label =~ '^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$') {
    error make { msg: $"invalid version label: ($label)" }
  }

  $"v($label)"
}

def is_prerelease [label: string] {
  $label =~ '^v[0-9]+\.[0-9]+\.[0-9]+-'
}
