terraform {

  required_version = ">= 1.8.3"

  backend "s3" {
  }

  required_providers {
    grafana = {
      source  = "grafana/grafana"
      version = ">= 3.7.0"
    }
  }
}
