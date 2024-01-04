set shell := ["nu", "-c"]

# List recipe
default:
  just --list

# Format files
fmt: fmt-toml

# Format toml files
fmt-toml:
  taplo fmt **.toml

gen-gql:
  graphql-client generate \
    --schema-path syndterm/gql/schema.json \
    --output-directory syndterm/src/client \
    --response-derives "Debug" \
    syndterm/gql/query.gql
