import '.base.just'

# Run linters
default: typo clippy

# Run clippy
[no-cd]
clippy:
    cargo clippy --all-features --tests --benches

# Run spell checker
[no-cd]
typo:
    nix build .#checks.{{arch()}}-{{os}}.typo --print-build-logs
