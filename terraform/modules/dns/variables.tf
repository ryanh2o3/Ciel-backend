variable "domain_name" {
  description = "Domain name (e.g., ciel-social.eu)"
  type        = string
}

variable "api_subdomain" {
  description = "Subdomain for API (e.g., api)"
  type        = string
  default     = "api"
}

variable "cdn_subdomain" {
  description = "Subdomain for CDN/media (e.g., media)"
  type        = string
  default     = "media"
}

variable "load_balancer_ip" {
  description = "Load balancer public IP address"
  type        = string
}

variable "cdn_endpoint" {
  description = "CDN endpoint URL"
  type        = string
  default     = ""
}

variable "enable_api_dns" {
  description = "Enable DNS records for API"
  type        = bool
  default     = true
}

variable "enable_cdn_dns" {
  description = "Enable DNS records for CDN"
  type        = bool
  default     = true
}

variable "enable_www_dns" {
  description = "Enable DNS records for www subdomain"
  type        = bool
  default     = true
}

variable "enable_root_dns" {
  description = "Enable DNS records for root domain"
  type        = bool
  default     = true
}

variable "enable_ssl" {
  description = "Enable SSL certificate provisioning"
  type        = bool
  default     = true
}

variable "load_balancer_id" {
  description = "Load balancer ID (required for SSL certificate provisioning)"
  type        = string
  default     = null
}