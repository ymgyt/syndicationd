set shell := ["nu", "-c"]

kvsd_user := "synduser"
github_pat := env_var_or_default("GH_PAT", "")
otlp_endpoint := env_var_or_default("OTEL_EXPORTER_OTLP_ENDPOINT", "")
loki_endpoint := env_var_or_default("LOKI_ENDPOINT", "")
term_dir := "crates/synd_term"
auth_dir := "crates/synd_auth"
feed_dir := "crates/synd_feed"
o11y_dir := "crates/synd_o11y"
api_dir := "crates/synd_api"

alias format := fmt
alias integration := integration-test
alias unused := machete

# List recipe
default:
    just --list

# Run check
check: typo
    nix flake check --all-systems --accept-flake-config

# Run spell checker
typo:
    typos

# Run audit
audit:
    cargo audit

# Format files
fmt: fmt-toml

# Run linter
lint:
    cargo clippy

# Format toml files
fmt-toml:
    taplo fmt --config taplo.toml **.toml

# Check unused dependencies
machete:
    cargo machete

# Run test
test *flags:
    cargo nextest run {{ flags }}

# Run integration test
integration-test:
    RUST_LOG="synd,integration,kvsd=info,info" cargo nextest run --package synd-term --features integration --test integration --no-capture 

# Update synd_api graphql schema
update-gql-schema:
    @graphql-client introspect-schema https://localhost:5959/graphql --no-ssl \
      --header 'authorization: github {{ github_pat }}' out> crates/synd_term/gql/schema.json

# Generate graphql code
gen-gql:
    graphql-client generate \
      --schema-path crates/synd_term/gql/schema.json \
      --output-directory crates/synd_term/src/client \
      --variables-derives "Debug" \
      --response-derives "Debug" \
      --custom-scalars-module "crate::client::scalar" \
      crates/synd_term/gql/query.gql

    graphql-client generate \
      --schema-path crates/synd_term/gql/schema.json \
      --output-directory crates/synd_term/src/client \
      --variables-derives "Debug" \
      --response-derives "Debug" \
      --custom-scalars-module "crate::client::scalar" \
      crates/synd_term/gql/mutation.gql

    graphql-client generate \
      --schema-path crates/synd_api/src/client/github/schema.json \
      --output-directory crates/synd_api/src/client/github \
      --variables-derives "Debug" \
      --response-derives "Debug" \
      crates/synd_api/src/client/github/query.gql

# Run kvsd
kvsd:
    cd ../kvsd; \
    KVSD_LOG=info cargo run server --disable-tls --config ../syndicationd/.dev/kvsd_config.yaml --username {{ kvsd_user }} --kvsd-dir ../syndicationd/.kvsd

# Run api
api *flags:
    do -i { ps | where name =~ "synd_api$" | first | kill $in.pid }
    cd crates/synd_api; \
      SYND_LOG="info,synd_api=debug" \
      OTEL_EXPORTER_OTLP_ENDPOINT={{ otlp_endpoint }} \
      OTEL_RESOURCE_ATTRIBUTES="service.namespace=syndlocal,deployment.environment=local" \
      cargo run \
        --features "introspection" -- \
        --kvsd-host 127.0.0.1 --kvsd-port 7379 --kvsd-username {{ kvsd_user }} --kvsd-password secret \
        --tls-cert ../../.dev/self_signed_certs/certificate.pem --tls-key ../../.dev/self_signed_certs/private_key.pem \
        --show-code-location=true --show-target=false --trace-sampler-ratio "1.0" {{ flags }}

# Run term
term *flags:
    cd crates/synd_term; cargo run -- --log /tmp/syndterm.log --endpoint https://localhost:5959 {{ flags }}

# Run opentelemetry-collector-contrib
@otelcol config:
    otelcontribcol --config=file:.dev/otelcol/{{ config }}

# Run backends
backend:
    zellij action new-tab --layout .dev/backend_layout.kdl

# Record demo
demo *flags:
    LC_ALL="en_US.UTF-8" LANG="en_US.UTF-8" nix run nixpkgs#asciinema -- rec demo.cast --overwrite "{{ flags }}"

# Convert demo to gif
demo2gif *flags:
    LC_ALL="en_US.UTF-8" LANG="en_US.UTF-8" nix run nixpkgs#asciinema-agg -- demo.cast assets/demo.gif {{ flags }}
    rm demo.cast

changelog-auth:
    GIT_CLIFF__GIT__TAG_PATTERN="synd-auth-v.*" \
    git cliff --include-path "{{ auth_dir }}/**" --include-path "crates/synd_authn/**" out> {{ auth_dir }}/CHANGELOG.md

changelog-o11y:
    GIT_CLIFF__GIT__TAG_PATTERN="synd-o11y-v.*" \
    git cliff --include-path "{{ o11y_dir }}/**" out> {{ o11y_dir }}/CHANGELOG.md

changelog-feed:
    GIT_CLIFF__GIT__TAG_PATTERN="synd-feed-v.*" \
    git cliff --include-path "{{ feed_dir }}/**" out> {{ feed_dir }}/CHANGELOG.md

changelog-term:
    GIT_CLIFF__GIT__TAG_PATTERN="synd-term-v.*" \
    git cliff --include-path "{{ term_dir }}/**" out> {{ term_dir }}/CHANGELOG.md

changelog-api:
    GIT_CLIFF__GIT__TAG_PATTERN="synd-api-v.*" \
    git cliff --include-path "{{ api_dir }}/**" out> {{ api_dir }}/CHANGELOG.md

# Release synd_auth
release-auth *flags: changelog-auth
    cargo release --package synd-auth {{ flags }}

release-o11y *flags: changelog-o11y
    cargo release --package synd-o11y {{ flags }}

# Release synd_feed
release-feed *flags: changelog-feed
    cargo release --package synd-feed {{ flags }}

# Release synd_term
release-term *flags: changelog-term
    cargo release --package synd-term {{ flags }}

release-api *flags: changelog-api
    cargo release --package synd-api {{ flags }}

dist-init:
    ~/.cargo/bin/cargo-dist dist init

dist-plan version:
    ~/.cargo/bin/cargo-dist dist plan --tag=synd-term-{{version}} --output-format=json

retag tag:
    git tag --delete {{ tag }}
    git push --delete origin {{ tag }}
    git tag {{ tag }}
    git push --tags

# Build and serve oranda server then watch
oranda-dev:
    oranda dev --verbose

# Generate oranda github actions workflow
oranda-gen:
    oranda generate ci --output-path .github/workflows/website.yaml --ci github
    echo "Make sure cp CNAME job add to workflow!!"
