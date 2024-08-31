import '.base.just'

# Update Cargo.toml [metadata.dist] section
[no-cd]
init:
    cargo dist init

# Show cargo-dist plan
[no-cd]
plan version:
    cargo dist plan --tag=synd-term-{{version}} --output-format=json

# Generate github action release workflow
[no-cd]
generate:
    cargo dist generate --mode ci
