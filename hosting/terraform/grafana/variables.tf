variable "grafana_url" {
  type        = string
  sensitive   = true
  description = "grafana instance url"
  default     = "https://ymgyt.grafana.net"
}

variable "grafana_sa_token" {
  type        = string
  sensitive   = true
  description = "grafana service account token"
}
