variable "project_id" {
  description = "Scaleway project ID"
  type        = string
}

variable "region" {
  description = "Scaleway region"
  type        = string
  default     = "fr-par"
}

variable "zone" {
  description = "Scaleway zone"
  type        = string
  default     = "fr-par-1"
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
}

variable "app_name" {
  description = "Application name"
  type        = string
  default     = "ciel"
}

variable "enable_load_balancer" {
  description = "Enable load balancer for API instances"
  type        = bool
  default     = true
}

variable "lb_type" {
  description = "Load balancer type"
  type        = string
  default     = "LB-S"
}

variable "enable_basic_waf" {
  description = "Enable basic L7 ACL protections on the load balancer"
  type        = bool
  default     = true
}

variable "blocked_ip_ranges" {
  description = "CIDR ranges to block at the load balancer (basic WAF/DDoS mitigation)"
  type        = list(string)
  default     = []
}

variable "enable_bastion" {
  description = "Enable bastion host for SSH access"
  type        = bool
  default     = false
}

variable "ssh_allowed_cidrs" {
  description = "CIDR blocks allowed to SSH when bastion is enabled"
  type        = list(string)
  default     = []
}

variable "bastion_instance_type" {
  description = "Bastion instance type"
  type        = string
  default     = "DEV1-S"
}

variable "private_network_cidr" {
  description = "CIDR block for private network"
  type        = string
  default     = "10.0.0.0/24"
}

variable "enable_public_gateway" {
  description = "Enable public gateway for outbound internet access"
  type        = bool
  default     = true
}

variable "public_gateway_type" {
  description = "Public gateway type"
  type        = string
  default     = "VPC-GW-S"
}

variable "enable_public_https" {
  description = "Open ports 80/443 on API security group for direct Caddy/Let's Encrypt SSL (no LB)"
  type        = bool
  default     = false
}

variable "ssl_certificate_ids" {
  description = "SSL certificate IDs for HTTPS termination"
  type        = list(string)
  default     = []
}

variable "health_check_path" {
  description = "Health check endpoint path"
  type        = string
  default     = "/health"
}

variable "backend_server_ips" {
  description = "List of backend server IPs to register with the load balancer"
  type        = list(string)
  default     = []
}

variable "reverse_dns_hostname" {
  description = "Hostname for reverse DNS on the public gateway IP (e.g., api.ciel-social.eu)"
  type        = string
  default     = null
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = list(string)
  default     = []
}
