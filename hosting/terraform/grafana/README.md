# Grafana

## Init project

### S3 Backend

```nu
$env.TF_BE_BUCKET  = "your_s3_bucket"
$env.TF_BE_KEY     = "state_file_path"
$env.TF_BE_PROFILE = "aws profile"
$env.TF_BE_REGION  = "aws region"

# Run terraform init with given backend configuration
just init-s3
```
