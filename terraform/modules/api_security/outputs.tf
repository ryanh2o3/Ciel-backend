# API Security Module Outputs

output "api_gateway_ip" {
  description = "API gateway public IP address"
  value       = var.enable_api_gateway ? scaleway_lb_ip.api_gateway[0].ip_address : null
}

output "api_gateway_id" {
  description = "API gateway load balancer ID"
  value       = var.enable_api_gateway ? scaleway_lb.api_gateway[0].id : null
}

output "api_key_secret_ids" {
  description = "Secret IDs for API keys"
  value       = scaleway_secret.api_keys[*].id
  sensitive   = true
}

output "security_group_id" {
  description = "API gateway security group ID"
  value       = var.enable_ip_restrictions ? scaleway_instance_security_group.api_gateway[0].id : null
}

output "allowed_ips" {
  description = "Configured allowed IP addresses"
  value       = var.allowed_ips
}