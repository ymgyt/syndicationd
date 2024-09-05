variable "grafana_url" {
  type        = string
  sensitive   = true
  description = "grafana instance url"
}

variable "grafana_sa_token" {
  type        = string
  sensitive   = true
  description = "grafana service account token"
}
