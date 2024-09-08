# Grafana

## Init project

```nu
# Configure credentials for grafana provider
$env.TF_VAR_grafana_url = "https://your.grafana.net"
$env.TF_VAR_grafana_sa_token = "[REDACTED]"
```

## Dashboards

Based on the JSON files stored in the [dashboards](./dashboards) directory, dashboards will be created under the Syndicationd folder in Grafana.
