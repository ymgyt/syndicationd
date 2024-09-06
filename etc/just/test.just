import '.base.just'

alias cov := coverage

# Run tests
default: unit integration

# Run unitest
[no-cd]
unit *flags:
    cargo nextest run {{ flags }}

# Run integration test by insta
[no-cd]
integration *test_filter:
    @nu etc/scripts/integration.nu {{ test_filter }}

# Run integration test with debugging
[no-cd]
integration-debug *case:
	RUST_LOG="synd=debug,octocrab=debug" SYND_LOG_LOCATION="true" \
	cargo nextest run \
		--package synd-term \
		--features integration \
		--test integration {{ case }} \
		--no-capture

# Run cargo insta review
[no-cd]
review:
    cargo insta review

# Generate test coverage
[linux]
coverage *flags:
    nix run nixpkgs#cargo-llvm-cov -- llvm-cov nextest \
        --all-features --open \
        --ignore-filename-regex '(generated/.*.rs|crates/ebpf/.*)' \
        {{ flags }}

[macos]
coverage:
    cargo llvm-cov nextest --all-features --open \
        --ignore-filename-regex '(integration_backend.rs|client/generated/.*.rs)'
