def main [] {
  # nix remove indexed output
  do --ignore-errors { nix profile remove syndicationd }

  nix profile install .
}
