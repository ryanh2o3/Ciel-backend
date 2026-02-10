# Generate random passwords if requested
resource "random_password" "db_password" {
  count   = var.generate_db_password ? 1 : 0
  length  = 32
  special = true
  override_special = "!@#$%^&*"
}

resource "random_password" "redis_password" {
  count   = var.generate_redis_password ? 1 : 0
  length  = 32
  special = false # Redis passwords work better without special chars
}

locals {
  db_password    = var.generate_db_password ? random_password.db_password[0].result : var.db_password
  redis_password = var.generate_redis_password ? random_password.redis_password[0].result : var.redis_password
}

# PASETO Access Key Secret
resource "scaleway_secret" "paseto_access_key" {
  name        = "${var.app_name}-paseto-access-key-${var.environment}"
  description = "PASETO access token signing key"
  project_id  = var.project_id
  region      = var.region
  tags        = var.tags
}

resource "scaleway_secret_version" "paseto_access_key" {
  secret_id = scaleway_secret.paseto_access_key.id
  data      = var.paseto_access_key
  region    = var.region
}

# PASETO Refresh Key Secret
resource "scaleway_secret" "paseto_refresh_key" {
  name        = "${var.app_name}-paseto-refresh-key-${var.environment}"
  description = "PASETO refresh token signing key"
  project_id  = var.project_id
  region      = var.region
  tags        = var.tags
}

resource "scaleway_secret_version" "paseto_refresh_key" {
  secret_id = scaleway_secret.paseto_refresh_key.id
  data      = var.paseto_refresh_key
  region    = var.region
}

# Admin Token Secret (optional)
resource "scaleway_secret" "admin_token" {
  count = var.admin_token != null && var.admin_token != "" ? 1 : 0

  name        = "${var.app_name}-admin-token-${var.environment}"
  description = "Admin API authentication token"
  project_id  = var.project_id
  region      = var.region
  tags        = var.tags
}

resource "scaleway_secret_version" "admin_token" {
  count = var.admin_token != null && var.admin_token != "" ? 1 : 0

  secret_id = scaleway_secret.admin_token[0].id
  data      = var.admin_token
  region    = var.region
}

# Database Password Secret
resource "scaleway_secret" "db_password" {
  name        = "${var.app_name}-db-password-${var.environment}"
  description = "Database password"
  project_id  = var.project_id
  region      = var.region
  tags        = var.tags
}

resource "scaleway_secret_version" "db_password" {
  secret_id = scaleway_secret.db_password.id
  data      = local.db_password
  region    = var.region
}

# Redis Password Secret
resource "scaleway_secret" "redis_password" {
  name        = "${var.app_name}-redis-password-${var.environment}"
  description = "Redis password"
  project_id  = var.project_id
  region      = var.region
  tags        = var.tags
}

resource "scaleway_secret_version" "redis_password" {
  secret_id = scaleway_secret.redis_password.id
  data      = local.redis_password
  region    = var.region
}

# S3 Access Key Secret
resource "scaleway_secret" "s3_access_key" {
  count = var.enable_s3_secrets ? 1 : 0

  name        = "${var.app_name}-s3-access-key-${var.environment}"
  description = "S3 access key"
  project_id  = var.project_id
  region      = var.region
  tags        = var.tags
}

resource "scaleway_secret_version" "s3_access_key" {
  count = var.enable_s3_secrets ? 1 : 0

  secret_id = scaleway_secret.s3_access_key[0].id
  data      = var.s3_access_key
  region    = var.region
}

# S3 Secret Key Secret
resource "scaleway_secret" "s3_secret_key" {
  count = var.enable_s3_secrets ? 1 : 0

  name        = "${var.app_name}-s3-secret-key-${var.environment}"
  description = "S3 secret key"
  project_id  = var.project_id
  region      = var.region
  tags        = var.tags
}

resource "scaleway_secret_version" "s3_secret_key" {
  count = var.enable_s3_secrets ? 1 : 0

  secret_id = scaleway_secret.s3_secret_key[0].id
  data      = var.s3_secret_key
  region    = var.region
}

# SQS Access Key Secret
resource "scaleway_secret" "sqs_access_key" {
  count = var.enable_sqs_secrets ? 1 : 0

  name        = "${var.app_name}-sqs-access-key-${var.environment}"
  description = "SQS access key"
  project_id  = var.project_id
  region      = var.region
  tags        = var.tags
}

resource "scaleway_secret_version" "sqs_access_key" {
  count = var.enable_sqs_secrets ? 1 : 0

  secret_id = scaleway_secret.sqs_access_key[0].id
  data      = var.sqs_access_key
  region    = var.region
}

# SQS Secret Key Secret
resource "scaleway_secret" "sqs_secret_key" {
  count = var.enable_sqs_secrets ? 1 : 0

  name        = "${var.app_name}-sqs-secret-key-${var.environment}"
  description = "SQS secret key"
  project_id  = var.project_id
  region      = var.region
  tags        = var.tags
}

resource "scaleway_secret_version" "sqs_secret_key" {
  count = var.enable_sqs_secrets ? 1 : 0

  secret_id = scaleway_secret.sqs_secret_key[0].id
  data      = var.sqs_secret_key
  region    = var.region
}
