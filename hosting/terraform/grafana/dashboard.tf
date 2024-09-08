resource "grafana_folder" "syndicationd" {
  title                        = "Syndicationd"
  prevent_destroy_if_not_empty = false
}

resource "grafana_dashboard" "synd_api" {
  for_each    = fileset("${path.module}/dashboards", "*.json")
  config_json = file("${path.module}/dashboards/${each.key}")
  folder      = grafana_folder.syndicationd.id
}
