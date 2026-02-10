output "instance_id" {
  description = "Database instance ID"
  value       = scaleway_rdb_instance.main.id
}

output "endpoint_ip" {
  description = "Database endpoint IP (via private network)"
  value       = length(scaleway_rdb_instance.main.private_network) > 0 ? scaleway_rdb_instance.main.private_network[0].ip : null
}

output "endpoint_port" {
  description = "Database endpoint port (via private network)"
  value       = length(scaleway_rdb_instance.main.private_network) > 0 ? scaleway_rdb_instance.main.private_network[0].port : null
}

output "private_endpoint" {
  description = "Database private network endpoint"
  value       = length(scaleway_rdb_instance.main.private_network) > 0 ? scaleway_rdb_instance.main.private_network[0].ip : null
}

output "primary_endpoint" {
  description = "Primary database endpoint (host:port)"
  value       = length(scaleway_rdb_instance.main.private_network) > 0 ? "${scaleway_rdb_instance.main.private_network[0].ip}:${scaleway_rdb_instance.main.private_network[0].port}" : null
}

output "read_endpoint" {
  description = "Read replica endpoint (first replica, host:port)"
  value       = length(scaleway_rdb_read_replica.main) > 0 && length(scaleway_rdb_read_replica.main[0].private_network) > 0 ? "${scaleway_rdb_read_replica.main[0].private_network[0].ip}:${scaleway_rdb_read_replica.main[0].private_network[0].port}" : null
}

output "database_name" {
  description = "Database name"
  value       = scaleway_rdb_database.main.name
}

output "database_user" {
  description = "Database user"
  value       = scaleway_rdb_user.app.name
}

output "database_url" {
  description = "Full database connection URL"
  value       = "postgres://${scaleway_rdb_user.app.name}:${var.db_user_password}@${scaleway_rdb_instance.main.private_network[0].ip}:${scaleway_rdb_instance.main.private_network[0].port}/${scaleway_rdb_database.main.name}?sslmode=require"
  sensitive   = true
}

output "read_replica_endpoints" {
  description = "Read replica endpoints"
  value = [
    for replica in scaleway_rdb_read_replica.main :
    length(replica.private_network) > 0 ? replica.private_network[0].ip : null
  ]
}

output "read_replica_urls" {
  description = "Read replica connection URLs"
  value = [
    for replica in scaleway_rdb_read_replica.main :
    "postgres://${scaleway_rdb_user.app.name}:${var.db_user_password}@${replica.private_network[0].ip}:${replica.private_network[0].port}/${scaleway_rdb_database.main.name}?sslmode=require"
  ]
  sensitive = true
}
