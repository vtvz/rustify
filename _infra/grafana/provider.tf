terraform {
  required_version = ">= 1.2.0"

  required_providers {
    grafana = {
      source  = "grafana/grafana"
      version = ">= 1.23.0"
    }
    jsonnet = {
      source  = "alxrem/jsonnet"
      version = ">= 2.1.0"
    }
  }
}

provider "grafana" {
  url  = var.grafana_url
  auth = var.grafana_auth
}

resource "grafana_data_source" "influxdb" {
  access_mode        = "proxy"
  basic_auth_enabled = false
  is_default         = false
  name               = "influxdb-cloud"
  type               = "influxdb"
  url                = var.influxdb_url
  password           = var.influxdb_token

  json_data {
    default_bucket = var.influxdb_default_bucket
    organization   = var.influxdb_organization
    version        = "Flux"
  }
}
