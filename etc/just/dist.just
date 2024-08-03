import '.base.just'

dist := '~/.cargo/bin/cargo-dist'

echo:
	echo {{dist}}

# Update Cargo.toml [metadata.dist] section
[no-cd]
init:
    {{ dist }} dist init

# Show cargo-dist plan
[no-cd]
plan version:
    {{ dist }} dist plan --tag=synd-term-{{version}} --output-format=json

# Generate github action release workflow
[no-cd]
generate:
    {{ dist }} dist generate --mode ci
