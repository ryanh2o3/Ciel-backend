output "vpc_id" {
  description = "VPC ID"
  value       = scaleway_vpc.main.id
}

output "private_network_id" {
  description = "Private network ID"
  value       = scaleway_vpc_private_network.main.id
}

output "api_security_group_id" {
  description = "API security group ID"
  value       = scaleway_instance_security_group.api.id
}

output "worker_security_group_id" {
  description = "Worker security group ID"
  value       = scaleway_instance_security_group.worker.id
}

output "redis_security_group_id" {
  description = "Redis security group ID"
  value       = scaleway_instance_security_group.redis.id
}

output "load_balancer_id" {
  description = "Load balancer ID"
  value       = var.enable_load_balancer ? scaleway_lb.api[0].id : null
}

output "load_balancer_ip" {
  description = "Load balancer public IP"
  value       = var.enable_load_balancer ? scaleway_lb_ip.api[0].ip_address : null
}

output "load_balancer_backend_id" {
  description = "Load balancer backend ID"
  value       = var.enable_load_balancer ? scaleway_lb_backend.api[0].id : null
}

output "public_gateway_id" {
  description = "Public gateway ID"
  value       = var.enable_public_gateway ? scaleway_vpc_public_gateway.main[0].id : null
}

output "public_gateway_ip" {
  description = "Public gateway IP address"
  value       = var.enable_public_gateway ? scaleway_vpc_public_gateway_ip.main[0].address : null
}

output "bastion_public_ip" {
  description = "Bastion host public IP"
  value       = var.enable_bastion ? try(scaleway_instance_server.bastion[0].public_ips[0].address, null) : null
}
