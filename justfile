set shell := ["nu", "-c"]
kvsd_user := "synduser"

# List recipe
default:
  just --list

# Run check
check:
  nix flake check --all-systems

# Format files
fmt: fmt-toml

# Format toml files
fmt-toml:
  taplo fmt **.toml

update-gql-schema:
  graphql-client introspect-schema http://localhost:5959/graphql \
    --header 'authorization: me' out> syndterm/gql/schema.json


gen-gql:
  graphql-client generate \
    --schema-path syndterm/gql/schema.json \
    --output-directory syndterm/src/client \
    --response-derives "Debug" \
    --custom-scalars-module "crate::client::scalar" \
    syndterm/gql/query.gql

  graphql-client generate \
    --schema-path syndterm/gql/schema.json \
    --output-directory syndterm/src/client \
    --response-derives "Debug" \
    --custom-scalars-module "crate::client::scalar" \
    syndterm/gql/mutation.gql

  graphql-client generate \
    --schema-path syndapi/src/client/github/schema.json \
    --output-directory syndapi/src/client/github \
    --response-derives "Debug" \
    syndapi/src/client/github/query.gql

# Run kvsd
kvsd:
  ~/.cargo/bin/kvsd server --disable-tls --config dev/kvsd_config.yaml --username {{kvsd_user}}

# Run api
api:
  cd syndapi; cargo run -- \
    --kvsd-host 127.0.0.1 --kvsd-port 7379 --kvsd-username {{kvsd_user}} --kvsd-password secret

# Run term
term:
  cd syndterm; cargo run -- --log /tmp/syndterm.log
