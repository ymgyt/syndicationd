set shell := ["nu", "-c"]

os := if os() == "macos" { "darwin" } else { "linux" }
