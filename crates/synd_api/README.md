# synd_api

syndicationd graphql api server

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
| `usecase`             | Counter   | usecase traffic per operation        |
| `cache.feed.count`    | Gauge     | feed cache entry count               |
| `cache.feed.size`     | Gauge     | feed cache size                      |


## Configurations

| Flag                    | Description                                    | Example                    |
| ---                     | ---                                            | ---                        |
| `--addr`                | Server bind address                            | `0.0.0.0`                  |
| `--port`                | Server bind port                               | `5959`                     |
| `--timeout`             | Request timeout                                | `30s`                      |
| `--body-limit-bytes`    | Request body limit                             | `2048`                     |
| `--concurrency-limit`   | Request concurrency limit                      | `100`                      |
| `--kvsd-host`           | Kvsd host                                      | `192.168.10.151`           |
| `--kvsd-port`           | Kvsd port                                      | `7379`                     |
| `--kvsd-username`       | Kvsd username                                  | `ferris`                   |
| `--kvsd-password`       | Kvsd password                                  | `secret`                   |
| `--tls-cert`            | Tls certificate path                           | `/path/to/certificate.pem` |
| `--tls-key`             | Tls private key path                           | `/path/to/secret.pem`      | 
| `--show-code-location`  | Show code location(foo.rs:10) in signals(logs) | `false`                    |
| `--show-target`         | Show tracing target(module) ins signals(logs)  | `true`                     |
| `--trace-sampler-ratio` | Trace sampler ratio                            | `1`                        |
| `--feed-cache-ttl`      | Feed entry cache TTL                           | `180min`                   |
| `-feed-cache-refresh-interval` | Feed entry cache refresh interval       | `120min`                   |


## Features

| Feature                | Description                          | Default |
| --                     | ---                                  | ---     |
| `introspection`        | Enable graphql introspection         | false   |
| `opentelemetry-stdout` | Enable opentelemetry stdout exporter | false   |
