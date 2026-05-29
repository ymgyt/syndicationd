# synd-api

syndicationd graphql api server

## Responsibility

`synd-api` is the transport boundary for syndicationd. It owns HTTP serving,
GraphQL schema wiring, authentication, request limits, shutdown, and
observability. Feed lifecycle behavior is delegated to `synd-registry`, and
SQLite storage adapters are provided by `synd-persistence`.

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


## Configurations

| Flag                    | Description                                    | Example                    |
| ---                     | ---                                            | ---                        |
| `--addr`                | Server bind address                            | `0.0.0.0`                  |
| `--port`                | Server bind port                               | `5959`                     |
| `--timeout`             | Request timeout                                | `30s`                      |
| `--body-limit-bytes`    | Request body limit                             | `2048`                     |
| `--concurrency-limit`   | Request concurrency limit                      | `100`                      |
| `--sqlite-db`           | Sqlite database path                           | `/path/to/synd.db`         |
| `--tls-cert`            | Tls certificate path                           | `/path/to/certificate.pem` |
| `--tls-key`             | Tls private key path                           | `/path/to/secret.pem`      | 
| `--show-code-location`  | Show code location(foo.rs:10) in signals(logs) | `false`                    |
| `--show-target`         | Show tracing target(module) ins signals(logs)  | `true`                     |
| `--trace-sampler-ratio` | Trace sampler ratio                            | `1`                        |
| `--default-feed-refresh-interval` | Default interval refresh policy for subscribed feeds | `120min` |


## Features

| Feature                | Description                          | Default |
| --                     | ---                                  | ---     |
| `introspection`        | Enable graphql introspection         | false   |
| `opentelemetry-stdout` | Enable opentelemetry stdout exporter | false   |
