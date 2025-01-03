# synd_kvsd

Syndicationd key value store daemon

## Configuration

| Flag                | Environment variable | Configuration file | Description | Default |
| ---                 | ---                  | ---                | ---         | ---     |
| `--connections-limit` | `KVSD_CONNECTIONS_LIMIT` | ??? | Max tcp connections | ??? |
| `--connection-buffer-bytes` | `KVSD_CONNECTION_BUFFER_BYTES` | ??? | Buffer bytes assigned to each connection | ??? |
| `--authenticate-timeout` | `KVSD_AUTHENTICATE_TIMEOUT` | ??? | Authenticate timeout | ??? |
| `--config` | `KVSD_CONFIG_FILE` | - | Configuration file path | ??? |
| `--bind-address` | `KVSD_BIND_ADDRESS` | ??? | Bind address | ??? |
| `--bind-port` | `KVSD_BIND_PORT` | ??? | Bind address | ??? |
| `--tls-cert` | `KVSD_TLS_CERT` | ??? | Tls server certificate path | ??? | 
| `--tls-key` | `KVSD_TLS_KEY` | ??? | Tls server private key path | ??? | 
| `--disable-tls` | `KVSD_DISABLE_TLS` |  ??? | Disable tls | ??? |

## Overview

![Request flow](../../etc/dot/dist/kvsd_architecture.svg)
