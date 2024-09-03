import '.base.just'

kvsd_user := "synduser"
otlp_endpoint := env_var_or_default("OTEL_EXPORTER_OTLP_ENDPOINT", "")
synd_endpoint := env_var_or_default("SYND_ENDPOINT", "https://localhost:5959")
loki_endpoint := env_var_or_default("LOKI_ENDPOINT", "")

# Run kvsd
[no-cd]
kvsd:
    # KVSD_LOG=info nix run github:ymgyt/kvsd/426ddaf5a6356551f0945b7ca3c48366580928d9 -- server \
        --disable-tls --config etc/dev/kvsd_config.yaml \
        --username {{ kvsd_user }} \
        --kvsd-dir .kvsd
    KVSD_LOG=info cargo run \
        --package synd-kvsd

[no-cd]
kvsd-local:
    cd ../kvsd; KVSD_LOG=info cargo run -- server \
        --disable-tls --config ../syndicationd/etc/dev/kvsd_config.yaml \
        --username {{ kvsd_user }} \
        --kvsd-dir .../syndicationd/kvsd

# Run api
[no-cd]
api *flags:
    try { ps | where name =~ "synd_api$" | first | kill $in.pid }
    cd crates/synd_api; \
      SYND_LOG="info,synd_api=debug" \
      OTEL_EXPORTER_OTLP_ENDPOINT={{ otlp_endpoint }} \
      OTEL_RESOURCE_ATTRIBUTES="service.namespace=syndlocal,deployment.environment=local" \
      cargo run --features opentelemetry-stdout,introspection -- \
        --kvsd-host 127.0.0.1 --kvsd-port 7379 --kvsd-username {{ kvsd_user }} --kvsd-password secret \
        --tls-cert ../../etc/dev/self_signed_certs/certificate.pem --tls-key ../../etc/dev/self_signed_certs/private_key.pem \
        --show-code-location=true --show-target=false --trace-sampler-ratio "1.0" {{ flags }}

# Run term
[no-cd]
term *flags:
	cd crates/synd_term; \
	SYND_ENDPOINT={{ synd_endpoint }} cargo run -- \
	--log /tmp/syndterm.log {{ flags }}

# Run opentelemetry-collector-contrib
[no-cd]
@otelcol config:
    otelcontribcol --config=file:etc/dev/otelcol/{{ config }}.yaml

# Run backends
[no-cd]
backend:
    zellij action new-tab --layout etc/dev/backend_layout.kdl
