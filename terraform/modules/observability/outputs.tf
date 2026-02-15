output "cockpit_enabled" {
  description = "Whether Cockpit is enabled"
  value       = var.enable_cockpit
}

output "grafana_url" {
  description = "Grafana dashboard URL (authenticate with Scaleway IAM credentials)"
  value       = var.enable_cockpit ? data.scaleway_cockpit_grafana.main[0].grafana_url : null
}

output "metrics_url" {
  description = "Metrics push endpoint"
  value       = var.enable_cockpit ? scaleway_cockpit_source.metrics[0].push_url : null
}

output "logs_url" {
  description = "Logs push endpoint"
  value       = var.enable_cockpit ? scaleway_cockpit_source.logs[0].push_url : null
}

output "traces_url" {
  description = "Traces push endpoint"
  value       = var.enable_cockpit ? scaleway_cockpit_source.traces[0].push_url : null
}

output "cockpit_token" {
  description = "Cockpit token for pushing metrics/logs"
  value       = var.enable_cockpit ? scaleway_cockpit_token.main[0].secret_key : null
  sensitive   = true
}
