terraform {

  required_version = ">= 1.8.3"

  cloud {
    organization = "ymgyt"

    workspaces {
      name = "grafana"
    }
  }

  required_providers {
    grafana = {
      source  = "grafana/grafana"
      version = ">= 3.7.0"
    }
  }
}

provider "grafana" {
  url  = var.grafana_url
  auth = var.grafana_sa_token
}
