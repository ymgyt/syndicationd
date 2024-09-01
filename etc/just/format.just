import '.base.just'

# Format files
default: toml cargo nix

# Format toml files
[no-cd]
toml:
    taplo fmt --config {{justfile_directory()}}/taplo.toml **.toml

# Run cargo fmt
[no-cd]
cargo:
	cargo fmt

# Run nix fmt
[no-cd]
nix:
	nix fmt
