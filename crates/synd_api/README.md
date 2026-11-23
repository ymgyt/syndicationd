# synd-api

syndicationd API service library

## Responsibility

`synd-api` is the transport boundary for syndicationd. It owns HTTP serving,
GraphQL schema wiring, authentication, request limits, shutdown, and
observability. It delegates feed lifecycle operations to `synd-registry`,
which also owns their SQLite storage.

This crate is a library. The local daemon process is started through
`synd daemon serve`.

## OpenTelemetry

synd-api is instrumented with OpenTelemetry.

### Traces

graphql request traces is exported

### Metrics

Following metrics are exported

| Metrics               | Meter     | Description                          |
| ---                   | ---       | ---                                  |
| `http.server.request` | Counter   | http request traffic per status code |
| `graphql.duration`    | Histogram | graphql latency                      |
| `registry.reconcile`  | Counter   | feed registry reconciliation attempts |


## Features

| Feature                | Description                          | Default |
| --                     | ---                                  | ---     |
| `introspection`        | Enable graphql introspection         | false   |
| `opentelemetry-stdout` | Enable opentelemetry stdout exporter | false   |
