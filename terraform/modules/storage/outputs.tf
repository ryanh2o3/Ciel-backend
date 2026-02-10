output "bucket_name" {
  description = "S3 bucket name"
  value       = scaleway_object_bucket.media.name
}

output "bucket_endpoint" {
  description = "S3 bucket endpoint"
  value       = scaleway_object_bucket.media.endpoint
}

output "bucket_region" {
  description = "S3 bucket region"
  value       = scaleway_object_bucket.media.region
}

output "s3_endpoint" {
  description = "S3 API endpoint"
  value       = "https://s3.${var.region}.scw.cloud"
}

output "s3_public_endpoint" {
  description = "Public URL for accessing media"
  value       = var.cdn_custom_domain != null ? "https://${var.cdn_custom_domain}" : "https://${scaleway_object_bucket.media.name}.s3.${var.region}.scw.cloud"
}

output "cdn_endpoint" {
  description = "CDN/public endpoint for media (alias for s3_public_endpoint)"
  value       = var.cdn_custom_domain != null ? "https://${var.cdn_custom_domain}" : "https://${scaleway_object_bucket.media.name}.s3.${var.region}.scw.cloud"
}

output "s3_access_key" {
  description = "S3 access key ID"
  value       = scaleway_iam_api_key.s3_access.access_key
  sensitive   = true
}

output "s3_secret_key" {
  description = "S3 secret access key"
  value       = scaleway_iam_api_key.s3_access.secret_key
  sensitive   = true
}

output "iam_application_id" {
  description = "IAM application ID"
  value       = scaleway_iam_application.s3_access.id
}

output "cdn_setup_notes" {
  description = "Notes for CDN setup"
  value = var.enable_cdn ? join("\n", [
    "CDN Setup Instructions:",
    "1. Go to Scaleway Console > Edge Services",
    "2. Create a new pipeline for bucket: ${scaleway_object_bucket.media.name}",
    "3. Configure caching rules for /processed/* prefix",
    "4. Optionally add custom domain: ${var.cdn_custom_domain != null ? var.cdn_custom_domain : "(not configured)"}",
    "5. Update S3_PUBLIC_ENDPOINT env var with CDN URL",
  ]) : "CDN disabled"
}
