# Grafana

## Init project

```nu
# Configure credentials for grafana provider
$env.TF_VAR_grafana_url = "https://your.grafana.net"
$env.TF_VAR_grafana_sa_token = "[REDACTED]"
```

### S3 Backend

```nu
$env.TF_BE_BUCKET  = "your_s3_bucket"
$env.TF_BE_KEY     = "state_file_path"
$env.TF_BE_PROFILE = "aws profile"
$env.TF_BE_REGION  = "aws region"

# Run terraform init with given backend configuration
just init-s3
```

## Dashboards

Based on the JSON files stored in the [dashboards](./dashboards) directory, dashboards will be created under the Syndicationd folder in Grafana.
