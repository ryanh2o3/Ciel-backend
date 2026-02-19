output "registry_namespace_id" {
  description = "Container registry namespace ID"
  value       = scaleway_registry_namespace.main.id
}

output "registry_endpoint" {
  description = "Container registry endpoint"
  value       = scaleway_registry_namespace.main.endpoint
}

# Instance outputs — unified across combined and multi-instance modes
output "api_instance_ids" {
  description = "API instance IDs"
  value = var.enable_combined_mode ? (
    scaleway_instance_server.combined[*].id
  ) : scaleway_instance_server.api[*].id
}

output "api_instance_ips" {
  description = "API instance private IPv4 addresses"
  value = var.enable_combined_mode ? (
    [for s in scaleway_instance_server.combined : try(
      [for ip in s.private_ips : ip.address if can(regex("^\\d+\\.\\d+\\.\\d+\\.\\d+$", ip.address))][0],
      null
    )]
  ) : [for s in scaleway_instance_server.api : try(
    [for ip in s.private_ips : ip.address if can(regex("^\\d+\\.\\d+\\.\\d+\\.\\d+$", ip.address))][0],
    null
  )]
}

output "api_instance_public_ips" {
  description = "API instance public IPs (if assigned)"
  value = var.enable_combined_mode ? (
    [for ip in scaleway_instance_ip.combined : ip.address]
  ) : [for s in scaleway_instance_server.api : try(s.public_ips[0].address, null)]
}

output "worker_instance_ids" {
  description = "Polling worker instance IDs (empty when using serverless worker)"
  value       = scaleway_instance_server.worker[*].id
}

output "worker_instance_ips" {
  description = "Polling worker instance private IPv4 addresses (empty when using serverless worker)"
  value = [for s in scaleway_instance_server.worker : try(
    [for ip in s.private_ips : ip.address if can(regex("^\\d+\\.\\d+\\.\\d+\\.\\d+$", ip.address))][0],
    null
  )]
}

# Serverless worker outputs
output "serverless_worker_endpoint" {
  description = "Serverless media worker endpoint URL"
  value       = var.enable_serverless_worker ? scaleway_container.media_processor[0].domain_name : null
}

output "serverless_worker_id" {
  description = "Serverless media worker container ID"
  value       = var.enable_serverless_worker ? scaleway_container.media_processor[0].id : null
}
