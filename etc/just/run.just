import '.base.just'

# Run term with in-process local api
[no-cd]
term *flags:
	cargo run --package synd -- \
	--log /tmp/syndterm.log {{ flags }}

# Run opentelemetry-collector-contrib
[no-cd]
@otelcol config:
    otelcontribcol --config=file:etc/dev/otelcol/{{ config }}.yaml
