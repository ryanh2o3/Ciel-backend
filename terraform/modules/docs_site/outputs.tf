output "bucket_name" {
  description = "Object Storage bucket name for static docs"
  value       = scaleway_object_bucket.docs.name
}

output "website_domain" {
  description = "Bucket website hostname (HTTP only; unused for public DNS when Edge is enabled)"
  value       = scaleway_object_bucket_website_configuration.docs.website_domain
}

output "dns_cname_target" {
  description = "Set Scaleway DNS CNAME `docs` → this host (Edge pipeline when enabled, else bucket website)"
  value       = length(scaleway_edge_services_pipeline.docs) > 0 ? "${scaleway_edge_services_pipeline.docs[0].id}.svc.edge.scw.cloud" : scaleway_object_bucket_website_configuration.docs.website_domain
}

output "edge_pipeline_id" {
  description = "Edge Services pipeline UUID (empty when Edge is disabled)"
  value       = length(scaleway_edge_services_pipeline.docs) > 0 ? scaleway_edge_services_pipeline.docs[0].id : ""
}

output "docs_https_url" {
  description = "Public documentation URL (HTTPS when Edge is enabled)"
  value = var.docs_fqdn != "" ? (
    local.edge_count > 0 ? "https://${var.docs_fqdn}" : "http://${var.docs_fqdn}"
  ) : scaleway_object_bucket_website_configuration.docs.website_endpoint
}

output "website_endpoint" {
  description = "Full website endpoint URL for the bucket"
  value       = scaleway_object_bucket_website_configuration.docs.website_endpoint
}

output "s3_api_endpoint" {
  description = "S3 API endpoint for aws CLI sync (same region as bucket)"
  value       = "https://s3.${var.region}.scw.cloud"
}

output "docs_deploy_access_key" {
  description = "Access key for CI (e.g. GitHub Actions) to sync to the docs bucket"
  value       = scaleway_iam_api_key.docs_deploy.access_key
}

output "docs_deploy_secret_key" {
  description = "Secret key for docs deploy — store in GitHub Actions secrets"
  value       = scaleway_iam_api_key.docs_deploy.secret_key
  sensitive   = true
}

locals {
  https_note_text = var.enable_edge_services && var.docs_fqdn != "" ? join(" ", [
    "Edge Services enabled: managed Let's Encrypt for ${var.docs_fqdn}.",
    "Point DNS CNAME docs to dns_cname_target (<pipeline-id>.svc.edge.scw.cloud).",
    "Certificate issuance may take a few minutes after apply.",
    ]) : join(" ", [
    "Edge Services disabled: dns_cname_target is the bucket website host (custom domain is usually HTTP-only).",
  ])
}

output "https_note" {
  description = "TLS / DNS notes for operators"
  value       = local.https_note_text
}
