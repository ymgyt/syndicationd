set shell := ["nu", "-c"]

kvsd_user := "synduser"
github_pat := env_var_or_default("GH_PAT", "")
synd_endpoint := env_var_or_default("SYND_ENDPOINT", "https://localhost:5959")
otlp_endpoint := env_var_or_default("OTEL_EXPORTER_OTLP_ENDPOINT", "")
loki_endpoint := env_var_or_default("LOKI_ENDPOINT", "")
term_dir := "crates/synd_term"
auth_dir := "crates/synd_auth"
feed_dir := "crates/synd_feed"
o11y_dir := "crates/synd_o11y"
api_dir := "crates/synd_api"

demo_tape := "assets/demo.tape"

arch := arch()
os := if os() == "macos" { "darwin" } else { "linux" }

alias format := fmt
alias unused := machete
alias licenses := license
alias cov := coverage

# List recipe
default:
    just --list

# Run check
check *flags:
    nix flake check --all-systems --accept-flake-config --print-build-logs --verbose {{ flags }}

# Run cargo check
c:
    cargo check --all-features --tests --benches

# Run spell checker
typo:
    nix build .#checks.{{arch}}-{{os}}.typo --print-build-logs

# Run audit
audit:
    nix build .#checks.{{arch}}-{{os}}.audit --print-build-logs 
    
# Update advisory db
update-advisory-db:
    nix flake lock --update-input advisory-db

# Format files
fmt: fmt-toml
    cargo fmt

# Run linter
lint:
    cargo clippy --all-features --tests --benches

# Format toml files
fmt-toml:
    taplo fmt --config taplo.toml **.toml

# Check unused dependencies
machete:
    cargo machete

# Run test
test *flags:
    cargo nextest run {{ flags }}

# Run integration test by insta
integration *test_filter:
    @nu scripts/integration.nu {{ test_filter }}

# Run integration test with debugging
integration-debug *case:
    RUST_LOG="synd=debug,octocrab=debug" SYND_LOG_LOCATION="true" \
        cargo nextest run --package synd-term --features integration --test integration {{ case }} --no-capture

# Run cargo insta review
review:
    cargo insta review
        
# Generate test coverage
[linux]
coverage *flags:
    nix run nixpkgs#cargo-llvm-cov -- llvm-cov nextest \
        --all-features --open \
        --ignore-filename-regex '(integration_backend.rs|client/generated/.*.rs)' \
        {{ flags }}

[macos]
coverage:
    cargo llvm-cov nextest --all-features --open \
        --ignore-filename-regex '(integration_backend.rs|client/generated/.*.rs)'

# Run benchmark
bench:
    cargo bench --package synd-term --bench render --features integration -- --verbose
    @start ./target/criterion/report/index.html

# Generate flamegraph
flamegraph:
    cargo flamegraph \
        --root --output target/flamegraph.svg \
        --package synd-term --bench render --features integration \
        -- --bench
    @start target/flamegraph.svg

# Update synd_api graphql schema
update-gql-schema:
    @graphql-client introspect-schema https://localhost:5959/graphql --no-ssl \
      --header 'authorization: github {{ github_pat }}' out> crates/synd_term/gql/schema.json

# Generate graphql code
gen-gql:
    graphql-client generate \
      --schema-path crates/synd_term/gql/schema.json \
      --output-directory crates/synd_term/src/client/generated \
      --variables-derives "Debug,Clone,PartialEq,Eq" \
      --response-derives "Debug,Clone,PartialEq,Eq" \
      --custom-scalars-module "crate::client::scalar" \
      crates/synd_term/gql/query.gql

    graphql-client generate \
      --schema-path crates/synd_term/gql/schema.json \
      --output-directory crates/synd_term/src/client/generated \
      --variables-derives "Debug,Clone,PartialEq,Eq" \
      --response-derives "Debug,Clone,PartialEq,Eq" \
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
    KVSD_LOG=info nix run github:ymgyt/kvsd/426ddaf5a6356551f0945b7ca3c48366580928d9 -- server \
        --disable-tls --config .dev/kvsd_config.yaml \
        --username {{ kvsd_user }} \
        --kvsd-dir .kvsd

# Run api
api *flags:
    try { ps | where name =~ "synd_api$" | first | kill $in.pid }
    cd crates/synd_api; \
      SYND_LOG="info,synd_api=debug" \
      OTEL_EXPORTER_OTLP_ENDPOINT={{ otlp_endpoint }} \
      OTEL_RESOURCE_ATTRIBUTES="service.namespace=syndlocal,deployment.environment=local" \
      cargo run --features opentelemetry-stdout,introspection -- \
        --kvsd-host 127.0.0.1 --kvsd-port 7379 --kvsd-username {{ kvsd_user }} --kvsd-password secret \
        --tls-cert ../../.dev/self_signed_certs/certificate.pem --tls-key ../../.dev/self_signed_certs/private_key.pem \
        --show-code-location=true --show-target=false --trace-sampler-ratio "1.0" {{ flags }}

# Run term
term *flags:
    cd crates/synd_term; SYND_ENDPOINT={{ synd_endpoint }} cargo run -- --log /tmp/syndterm.log {{ flags }}

# Run opentelemetry-collector-contrib
@otelcol config:
    otelcontribcol --config=file:.dev/otelcol/{{ config }}

# Run backends
backend:
    zellij action new-tab --layout .dev/backend_layout.kdl

# Record demo
demo *flags:
    LC_ALL="en_US.UTF-8" LANG="en_US.UTF-8" nix run nixpkgs#asciinema -- rec demo.cast --overwrite {{ flags }}

# Convert demo to gif
# --rederer=resvg need to render nerd fonts and CJK
demo2gif *flags:
    LC_ALL="en_US.UTF-8" LANG="en_US.UTF-8" nix run nixpkgs#asciinema-agg -- demo.cast assets/demo.gif --renderer=resvg {{ flags }}
    rm demo.cast

demo-vhs-record *flags:
    cat assets/vhs_settings.tape out> {{ demo_tape }}
    nix run nixpkgs#vhs -- record --shell nu out>> {{ demo_tape }}

demo-vhs-gif:
    nix run nixpkgs#vhs -- {{ demo_tape }}

demo-vhs-gif-short:
    nix run nixpkgs#vhs -- assets/demo_short.tape --output assets/demo_short.gif

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

# Update Cargo.toml [metadata.dist] section
dist-init:
    ~/.cargo/bin/cargo-dist dist init

dist-plan version:
    ~/.cargo/bin/cargo-dist dist plan --tag=synd-term-{{version}} --output-format=json

# Generate github action release workflow
dist-generate:
    ~/.cargo/bin/cargo-dist dist generate --mode ci

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

# Build and open rustdoc
doc:
    cargo doc --open --no-deps

# Generate dependencies licenses
license: 
    cargo bundle-licenses --format toml --output THIRDPARTY.toml

# Check dependencies licenses
license-check:
    try { RUST_LOG=error cargo bundle-licenses --format toml --output __CHECK --previous THIRDPARTY.toml --check-previous }
    rm __CHECK

# Validate codecov.yaml
codecov-validate:
    open codecov.yaml | to yaml | http post https://codecov.io/validate $in

# Login ghcr
docker-login:
    $env.GHCR_PAT | docker login ghcr.io -u USERNAME --password-stdin
    
# Reinstall synd-term
reinstall:
    @nu scripts/reinstall.nu

# Build ebpf program
build-ebpf *flags:
    nix develop .#ebpf --command cargo task build-ebpf {{ flags }}
