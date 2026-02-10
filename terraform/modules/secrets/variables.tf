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

variable "paseto_access_key" {
  description = "PASETO access token key (base64 encoded 32 bytes)"
  type        = string
  sensitive   = true
}

variable "paseto_refresh_key" {
  description = "PASETO refresh token key (base64 encoded 32 bytes)"
  type        = string
  sensitive   = true
}

variable "admin_token" {
  description = "Admin API token"
  type        = string
  sensitive   = true
  default     = ""
}

variable "generate_db_password" {
  description = "Generate a random database password"
  type        = bool
  default     = true
}

variable "db_password" {
  description = "Database password (if not generating)"
  type        = string
  sensitive   = true
  default     = ""
}

variable "generate_redis_password" {
  description = "Generate a random Redis password"
  type        = bool
  default     = true
}

variable "redis_password" {
  description = "Redis password (if not generating)"
  type        = string
  sensitive   = true
  default     = ""
}

variable "s3_access_key" {
  description = "S3 access key"
  type        = string
  sensitive   = true
  default     = ""
}

variable "s3_secret_key" {
  description = "S3 secret key"
  type        = string
  sensitive   = true
  default     = ""
}

variable "sqs_access_key" {
  description = "SQS access key"
  type        = string
  sensitive   = true
  default     = ""
}

variable "sqs_secret_key" {
  description = "SQS secret key"
  type        = string
  sensitive   = true
  default     = ""
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = list(string)
  default     = []
}
