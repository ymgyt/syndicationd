set shell := ["nu", "-c"]
kvsd_user := "synduser"
github_pat := env_var_or_default("GH_PAT", "")

# List recipe
default:
  just --list

# Run check
check:
  nix flake check --all-systems

# Format files
fmt: fmt-toml

lint:
  cargo clippy

# Format toml files
fmt-toml:
  taplo fmt **.toml

# Run integration test
integration:
  RUST_LOG="synd_term,integration=debug" cargo nextest run --package syndterm --features integration --test integration --no-capture 

update-gql-schema:
  @graphql-client introspect-schema http://localhost:5959/graphql \
    --header 'authorization: github {{github_pat}}' out> crates/synd_term/gql/schema.json


gen-gql:
  graphql-client generate \
    --schema-path crates/syndterm/gql/schema.json \
    --output-directory crates/syndterm/src/client \
    --response-derives "Debug" \
    --custom-scalars-module "crate::client::scalar" \
    crates/syndterm/gql/query.gql

  graphql-client generate \
    --schema-path crates/syndterm/gql/schema.json \
    --output-directory crates/syndterm/src/client \
    --response-derives "Debug" \
    --custom-scalars-module "crate::client::scalar" \
    crates/syndterm/gql/mutation.gql

  graphql-client generate \
    --schema-path crates/synd_api/src/client/github/schema.json \
    --output-directory crates/synd_api/src/client/github \
    --response-derives "Debug" \
    crates/synd_api/src/client/github/query.gql

# Run kvsd
kvsd:
  ~/.cargo/bin/kvsd server --disable-tls --config dev/kvsd_config.yaml --username {{kvsd_user}}

# Run api
api:
  cd crates/synd_api; RUST_LOG="info" cargo run --features "introspection" -- \
    --kvsd-host 127.0.0.1 --kvsd-port 7379 --kvsd-username {{kvsd_user}} --kvsd-password secret

# Run term
term:
  cd crates/synd_term; cargo run -- --log /tmp/syndterm.log
