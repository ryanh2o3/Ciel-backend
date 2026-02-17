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

# Container Registry
variable "registry_is_public" {
  description = "Whether the container registry is public"
  type        = bool
  default     = false
}

# ---- Combined Mode (single instance: API + Redis) ----

variable "enable_combined_mode" {
  description = "Run API + Redis on a single instance instead of separate instances"
  type        = bool
  default     = false
}

variable "combined_instance_type" {
  description = "Instance type for the combined (API + Redis) instance"
  type        = string
  default     = "DEV1-M"
}

variable "embedded_redis_maxmemory_mb" {
  description = "Max memory for embedded Redis in combined mode (MB)"
  type        = number
  default     = 512
}

# ---- Multi-Instance Mode ----

variable "api_instance_count" {
  description = "Number of API instances (ignored in combined mode)"
  type        = number
  default     = 1
}

variable "api_instance_type" {
  description = "API instance type (ignored in combined mode)"
  type        = string
  default     = "DEV1-S"
}

variable "worker_instance_count" {
  description = "Number of polling worker instances (set 0 when using serverless worker)"
  type        = number
  default     = 0
}

variable "worker_instance_type" {
  description = "Worker instance type"
  type        = string
  default     = "DEV1-S"
}

# ---- Serverless Worker (event-driven media processing) ----

variable "enable_serverless_worker" {
  description = "Deploy a Serverless Container for media processing, triggered by SQS"
  type        = bool
  default     = true
}

variable "serverless_worker_cpu" {
  description = "vCPU limit for serverless worker (millicores, 1000 = 1 vCPU)"
  type        = number
  default     = 1000
}

variable "serverless_worker_memory" {
  description = "Memory limit for serverless worker (MB)"
  type        = number
  default     = 512
}

variable "serverless_worker_min_scale" {
  description = "Minimum number of serverless worker instances (0 = scale to zero)"
  type        = number
  default     = 0
}

variable "serverless_worker_max_scale" {
  description = "Maximum number of concurrent serverless worker instances"
  type        = number
  default     = 5
}

variable "serverless_worker_timeout" {
  description = "Timeout for serverless worker requests in seconds"
  type        = number
  default     = 300
}

# Secrets for the serverless container (passed as env vars at deploy time)
variable "serverless_database_url" {
  description = "Full DATABASE_URL for the serverless worker container"
  type        = string
  default     = ""
  sensitive   = true
}

variable "serverless_s3_access_key" {
  description = "S3 access key for the serverless worker container"
  type        = string
  default     = ""
  sensitive   = true
}

variable "serverless_s3_secret_key" {
  description = "S3 secret key for the serverless worker container"
  type        = string
  default     = ""
  sensitive   = true
}

# Container Image
variable "container_image" {
  description = "Full container image path without tag (e.g., rg.fr-par.scw.cloud/ciel-social/ciel-backend)"
  type        = string
}

variable "container_image_tag" {
  description = "Docker image tag to deploy"
  type        = string
  default     = "stable"
}

# Network
variable "private_network_id" {
  description = "Private network ID"
  type        = string
}

variable "api_security_group_id" {
  description = "API security group ID"
  type        = string
}

variable "worker_security_group_id" {
  description = "Worker security group ID (only needed if worker_instance_count > 0)"
  type        = string
  default     = ""
}


# Environment Variables
variable "db_host" {
  description = "Database host (private endpoint)"
  type        = string
}

variable "db_port" {
  description = "Database port"
  type        = number
  default     = 5432
}

variable "db_name" {
  description = "Database name"
  type        = string
}

variable "db_user" {
  description = "Database user"
  type        = string
}

variable "db_password" {
  description = "Database password"
  type        = string
  sensitive   = true
}

variable "redis_host" {
  description = "Redis host (ignored in combined mode)"
  type        = string
  default     = ""
}

variable "redis_port" {
  description = "Redis port (ignored in combined mode)"
  type        = number
  default     = 6379
}

variable "redis_use_tls" {
  description = "Use TLS for Redis connections (ignored in combined mode)"
  type        = bool
  default     = false
}

variable "redis_password" {
  description = "Redis password"
  type        = string
  sensitive   = true
}

variable "s3_endpoint" {
  description = "S3 endpoint URL"
  type        = string
}

variable "s3_region" {
  description = "S3 region"
  type        = string
  default     = "fr-par"
}

variable "s3_bucket" {
  description = "S3 bucket name"
  type        = string
}

variable "s3_public_endpoint" {
  description = "S3 public/CDN endpoint for serving media"
  type        = string
}

variable "s3_access_key" {
  description = "S3 access key"
  type        = string
  sensitive   = true
}

variable "s3_secret_key" {
  description = "S3 secret key"
  type        = string
  sensitive   = true
}

variable "queue_endpoint" {
  description = "SQS queue endpoint"
  type        = string
}

variable "queue_region" {
  description = "SQS queue region"
  type        = string
  default     = "fr-par"
}

variable "queue_name" {
  description = "SQS queue name"
  type        = string
}

variable "sqs_access_key" {
  description = "SQS access key"
  type        = string
  sensitive   = true
}

variable "sqs_secret_key" {
  description = "SQS secret key"
  type        = string
  sensitive   = true
}

variable "paseto_access_key" {
  description = "PASETO access key"
  type        = string
  sensitive   = true
}

variable "paseto_refresh_key" {
  description = "PASETO refresh key"
  type        = string
  sensitive   = true
}

variable "admin_token" {
  description = "Admin token (optional)"
  type        = string
  default     = ""
  sensitive   = true
}


variable "http_addr" {
  description = "HTTP listen address"
  type        = string
  default     = "0.0.0.0:8080"
}

variable "rust_log" {
  description = "Rust log level"
  type        = string
  default     = "info"
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = list(string)
  default     = []
}

variable "runtime_iam_application_name" {
  description = "IAM application name for runtime instance access"
  type        = string
  default     = "ciel-runtime"
}
