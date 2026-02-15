# Input variables for dev environment

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
  default     = "dev-stable"
}

variable "ssh_allowed_cidrs" {
  description = "CIDR blocks allowed to SSH to dev bastion"
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
  description = "Enable DNS module for dev environment"
  type        = bool
  default     = true
}

variable "scw_secret_key" {
  description = "Scaleway secret key (for IAM-scoped cloud-init secret fetching)"
  type        = string
  sensitive   = true
}

variable "alert_contact_emails" {
  description = "Email addresses for alerting"
  type        = list(string)
  default     = []
}
