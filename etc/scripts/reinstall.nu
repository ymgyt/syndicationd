def main [] {
  for item in (nix profile list --json | from json | get elements | enumerate) {
    let index = $item | get index
    let store_path = $item | get item.storePaths | first

    if ($store_path | str contains 'synd-term') {
      print $"Remove following derivation"
      print ($item | get item)
      let answer = (["yes" "no" ] | input list)
      if ($answer == "yes") {
        nix profile remove $index
      }
    }
  }

  nix profile install .
}
