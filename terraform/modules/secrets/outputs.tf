output "paseto_access_key" {
  description = "PASETO access key"
  value       = var.paseto_access_key
  sensitive   = true
}

output "paseto_refresh_key" {
  description = "PASETO refresh key"
  value       = var.paseto_refresh_key
  sensitive   = true
}

output "admin_token" {
  description = "Admin token"
  value       = var.admin_token
  sensitive   = true
}

output "db_password" {
  description = "Database password"
  value       = local.db_password
  sensitive   = true
}

output "redis_password" {
  description = "Redis password"
  value       = local.redis_password
  sensitive   = true
}

output "paseto_access_key_secret_id" {
  description = "Secret Manager ID for PASETO access key"
  value       = scaleway_secret.paseto_access_key.id
}

output "paseto_refresh_key_secret_id" {
  description = "Secret Manager ID for PASETO refresh key"
  value       = scaleway_secret.paseto_refresh_key.id
}

output "admin_token_secret_id" {
  description = "Secret Manager ID for admin token"
  value       = var.admin_token != null && var.admin_token != "" ? scaleway_secret.admin_token[0].id : null
}

output "db_password_secret_id" {
  description = "Secret Manager ID for database password"
  value       = scaleway_secret.db_password.id
}

output "redis_password_secret_id" {
  description = "Secret Manager ID for Redis password"
  value       = scaleway_secret.redis_password.id
}

output "s3_access_key_secret_id" {
  description = "Secret Manager ID for S3 access key"
  value       = var.enable_s3_secrets ? scaleway_secret.s3_access_key[0].id : null
}

output "s3_secret_key_secret_id" {
  description = "Secret Manager ID for S3 secret key"
  value       = var.enable_s3_secrets ? scaleway_secret.s3_secret_key[0].id : null
}

output "sqs_access_key_secret_id" {
  description = "Secret Manager ID for SQS access key"
  value       = var.enable_sqs_secrets ? scaleway_secret.sqs_access_key[0].id : null
}

output "sqs_secret_key_secret_id" {
  description = "Secret Manager ID for SQS secret key"
  value       = var.enable_sqs_secrets ? scaleway_secret.sqs_secret_key[0].id : null
}
