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

variable "db_node_type" {
  description = "Database instance type"
  type        = string
  default     = "DB-DEV-S"
}

variable "db_engine" {
  description = "Database engine version"
  type        = string
  default     = "PostgreSQL-16"
}

variable "db_name" {
  description = "Database name"
  type        = string
  default     = "ciel"
}

variable "db_user" {
  description = "Database user name"
  type        = string
  default     = "ciel_app"
}

variable "db_admin_password" {
  description = "Database admin password (for the default admin user)"
  type        = string
  sensitive   = true
}

variable "db_user_password" {
  description = "Database application user password"
  type        = string
  sensitive   = true
}

variable "volume_size_in_gb" {
  description = "Database volume size in GB"
  type        = number
  default     = 10
}

variable "enable_ha" {
  description = "Enable high availability"
  type        = bool
  default     = false
}

variable "read_replica_count" {
  description = "Number of read replicas"
  type        = number
  default     = 0
}

variable "private_network_id" {
  description = "Private network ID for database endpoint"
  type        = string
}

variable "backup_schedule_frequency" {
  description = "Backup frequency in hours"
  type        = number
  default     = 24
}

variable "backup_schedule_retention" {
  description = "Number of backups to retain"
  type        = number
  default     = 7
}

variable "db_settings" {
  description = "PostgreSQL settings"
  type        = map(string)
  default = {
    work_mem              = "4"
    max_connections       = "100"
    effective_cache_size  = "768"
  }
}

variable "db_user_permission" {
  description = "Database permissions for application user"
  type        = string
  default     = "readwrite"
}

variable "volume_type" {
  description = "Database volume type (sbs_5k or sbs_15k). bssd is deprecated."
  type        = string
  default     = "sbs_5k"
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = list(string)
  default     = []
}
