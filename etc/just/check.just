import '.base.just'

# Run nix flake check
[no-cd]
default *flags:
	nix flake check \
		--all-systems \
		--accept-flake-config \
		--print-build-logs \
		--verbose {{ flags }}

# Run cargo check
[no-cd]
cargo:
    cargo check --all-features --tests --benches

