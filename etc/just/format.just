import '.base.just'

# Format files
default: toml cargo

# Format toml files
[no-cd]
toml:
    taplo fmt --config {{justfile_directory()}}/taplo.toml **.toml

# Run cargo fmt
[no-cd]
cargo:
	cargo fmt

