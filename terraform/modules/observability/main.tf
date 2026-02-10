# Enable Cockpit for the project
resource "scaleway_cockpit" "main" {
  count = var.enable_cockpit ? 1 : 0

  project_id = var.project_id
}

# Cockpit token for pushing metrics
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

  depends_on = [scaleway_cockpit.main]
}

# Grafana user for dashboard access
resource "scaleway_cockpit_grafana_user" "main" {
  count = var.enable_cockpit ? 1 : 0

  project_id = var.project_id
  login      = "${var.app_name}-${var.environment}"
  role       = var.grafana_user_role

  depends_on = [scaleway_cockpit.main]
}

# Data source to get Cockpit endpoints
data "scaleway_cockpit" "main" {
  count = var.enable_cockpit ? 1 : 0

  project_id = var.project_id

  depends_on = [scaleway_cockpit.main]
}

# Alert manager contact points
resource "scaleway_cockpit_alert_manager" "main" {
  count = var.enable_cockpit && var.enable_alerts ? 1 : 0

  project_id = var.project_id
  region     = var.region

  enable_managed_alerts = true

  dynamic "contact_points" {
    for_each = var.alert_contact_emails
    content {
      email = contact_points.value
    }
  }

  depends_on = [scaleway_cockpit.main]
}
