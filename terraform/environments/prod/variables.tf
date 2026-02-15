# Input variables for production environment

variable "project_id" {
  description = "Scaleway project ID"
  type        = string
  sensitive   = true
}

variable "organization_id" {
  description = "Scaleway organization ID"
  type        = string
  sensitive   = true
}

variable "zone" {
  description = "Scaleway zone"
  type        = string
  default     = "fr-par-1"
}

variable "region" {
  description = "Scaleway region"
  type        = string
  default     = "fr-par"
}


# Database credentials
variable "db_admin_password" {
  description = "Database admin password"
  type        = string
  sensitive   = true
}

variable "db_user_password" {
  description = "Database user password"
  type        = string
  sensitive   = true
}

# Redis password
variable "redis_password" {
  description = "Redis password"
  type        = string
  sensitive   = true
}

# PASETO keys for authentication
variable "paseto_access_key" {
  description = "PASETO access key (base64, 32 bytes)"
  type        = string
  sensitive   = true
}

variable "paseto_refresh_key" {
  description = "PASETO refresh key (base64, 32 bytes)"
  type        = string
  sensitive   = true
}

# Admin token (optional)
variable "admin_token" {
  description = "Admin token for initial setup"
  type        = string
  sensitive   = true
  default     = null
}

# Container image
variable "container_image_tag" {
  description = "Docker image tag to deploy"
  type        = string
  default     = "stable"
}

variable "ssh_allowed_cidrs" {
  description = "CIDR blocks allowed to SSH when bastion is enabled"
  type        = list(string)
  default     = []
}

# Observability alerts
variable "scw_secret_key" {
  description = "Scaleway secret key (passed by CI, used for provider auth)"
  type        = string
  sensitive   = true
  default     = null
}

variable "alert_contact_emails" {
  description = "Alert contact email addresses"
  type        = list(string)
  default     = []
}

# DNS Configuration
variable "domain_name" {
  description = "Domain name for DNS records (e.g., ciel-social.eu)"
  type        = string
  default     = "ciel-social.eu"
}

variable "enable_dns" {
  description = "Enable DNS module for production environment"
  type        = bool
  default     = true
}

# API Security Configuration
variable "enable_ip_restrictions" {
  description = "Enable IP-based access restrictions for API"
  type        = bool
  default     = false
}

variable "allowed_ips" {
  description = "List of allowed IP addresses for API access"
  type        = list(string)
  default     = []
}

variable "api_keys" {
  description = "API keys for application authentication"
  type        = list(string)
  default     = []
  sensitive   = true
}
