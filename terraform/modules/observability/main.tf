# Cockpit data sources (replaces deprecated scaleway_cockpit resource)
resource "scaleway_cockpit_source" "metrics" {
  count = var.enable_cockpit ? 1 : 0

  project_id     = var.project_id
  name           = "${var.app_name}-metrics-${var.environment}"
  type           = "metrics"
  retention_days = var.retention_days
}

resource "scaleway_cockpit_source" "logs" {
  count = var.enable_cockpit ? 1 : 0

  project_id     = var.project_id
  name           = "${var.app_name}-logs-${var.environment}"
  type           = "logs"
  retention_days = var.retention_days
}

resource "scaleway_cockpit_source" "traces" {
  count = var.enable_cockpit ? 1 : 0

  project_id     = var.project_id
  name           = "${var.app_name}-traces-${var.environment}"
  type           = "traces"
  retention_days = var.retention_days
}

# Cockpit token for pushing metrics/logs/traces
resource "scaleway_cockpit_token" "main" {
  count = var.enable_cockpit ? 1 : 0

  project_id = var.project_id
  name       = "${var.app_name}-metrics-${var.environment}"

  scopes {
    query_metrics = true
    write_metrics = true
    query_logs    = true
    write_logs    = true
    query_traces  = true
    write_traces  = true
  }
}

# Grafana access via IAM (replaces deprecated scaleway_cockpit_grafana_user)
data "scaleway_cockpit_grafana" "main" {
  count = var.enable_cockpit ? 1 : 0

  project_id = var.project_id
}

# Fetch all available preconfigured alerts
data "scaleway_cockpit_preconfigured_alert" "all" {
  count = var.enable_cockpit && var.enable_alerts ? 1 : 0

  project_id = var.project_id
  region     = var.region
}

# Alert manager contact points
resource "scaleway_cockpit_alert_manager" "main" {
  count = var.enable_cockpit && var.enable_alerts ? 1 : 0

  project_id = var.project_id
  region     = var.region

  preconfigured_alert_ids = [
    for alert in data.scaleway_cockpit_preconfigured_alert.all[0].alerts :
    alert.preconfigured_rule_id
  ]

  dynamic "contact_points" {
    for_each = var.alert_contact_emails
    content {
      email = contact_points.value
    }
  }
}
