locals {
  jsonnet_vars = {
    loki     = "grafanacloud-logs"
    influxdb = grafana_data_source.influxdb.uid
  }
}

data "jsonnet_file" "metrics" {
  source  = "${path.module}/jsonnet/dashboard.jsonnet"
  ext_str = local.jsonnet_vars
}

resource "grafana_dashboard" "metrics" {
  config_json = data.jsonnet_file.metrics.rendered
}
