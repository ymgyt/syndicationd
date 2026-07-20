import '.base.just'

# Update Cargo.toml [metadata.dist] section
[no-cd]
init *flags:
    dist init --allow-dirty {{flags}}

# Show cargo-dist plan
[no-cd]
plan tag:
    dist plan --tag={{tag}} --output-format=json

# Generate github action release workflow
[no-cd]
generate:
    dist generate --mode ci
