# Outputs for dev environment

output "api_instance_ips" {
  description = "Private IPs of API/combined instances"
  value       = module.compute.api_instance_ips
}

output "api_instance_public_ips" {
  description = "Public IPs of API/combined instances"
  value       = module.compute.api_instance_public_ips
}

output "database_url" {
  description = "Database connection URL"
  value       = module.database.database_url
  sensitive   = true
}

output "s3_bucket_name" {
  description = "S3 bucket name for media storage"
  value       = module.storage.bucket_name
}

output "s3_access_key" {
  description = "S3 access key"
  value       = module.storage.s3_access_key
  sensitive   = true
}

output "s3_secret_key" {
  description = "S3 secret key"
  value       = module.storage.s3_secret_key
  sensitive   = true
}

output "queue_endpoint" {
  description = "SQS queue endpoint"
  value       = module.messaging.queue_endpoint
}

output "queue_name" {
  description = "SQS queue name"
  value       = module.messaging.queue_name
}

output "sqs_access_key" {
  description = "SQS access key"
  value       = module.messaging.sqs_access_key
  sensitive   = true
}

output "sqs_secret_key" {
  description = "SQS secret key"
  value       = module.messaging.sqs_secret_key
  sensitive   = true
}

output "bastion_ip" {
  description = "Bastion host public IP (if enabled)"
  value       = module.networking.bastion_public_ip
}

output "private_network_id" {
  description = "Private network ID"
  value       = module.networking.private_network_id
}

# Serverless worker
output "serverless_worker_endpoint" {
  description = "Serverless media worker endpoint URL"
  value       = module.compute.serverless_worker_endpoint
}

# DNS Outputs (if enabled)
output "api_dns_record" {
  description = "API DNS record"
  value       = var.enable_dns ? module.dns[0].api_dns_record : null
}

output "cdn_dns_record" {
  description = "CDN DNS record"
  value       = var.enable_dns ? module.dns[0].cdn_dns_record : null
}

output "docs_bucket_name" {
  description = "Static documentation site bucket (GitHub Actions DOCS_BUCKET_NAME)"
  value       = var.enable_docs_hosting ? module.docs_site[0].bucket_name : null
}

output "docs_website_domain" {
  description = "Bucket website hostname (legacy; prefer docs_dns_cname_target)"
  value       = var.enable_docs_hosting ? module.docs_site[0].website_domain : null
}

output "docs_dns_cname_target" {
  description = "CNAME target for docs subdomain (Edge .svc.edge.scw.cloud or bucket website)"
  value       = var.enable_docs_hosting ? module.docs_site[0].dns_cname_target : null
}

output "docs_public_url" {
  description = "URL for the documentation site"
  value       = var.enable_docs_hosting ? module.docs_site[0].docs_https_url : null
}

output "docs_edge_pipeline_id" {
  description = "Edge Services pipeline ID for docs (empty if Edge disabled)"
  value       = var.enable_docs_hosting ? module.docs_site[0].edge_pipeline_id : null
}

output "docs_s3_api_endpoint" {
  description = "S3 endpoint URL for aws s3 sync (e.g. https://s3.fr-par.scw.cloud)"
  value       = var.enable_docs_hosting ? module.docs_site[0].s3_api_endpoint : null
}

output "docs_deploy_access_key" {
  description = "IAM access key for uploading docs (sensitive: pair with secret in CI)"
  value       = var.enable_docs_hosting ? module.docs_site[0].docs_deploy_access_key : null
  sensitive   = true
}

output "docs_deploy_secret_key" {
  description = "IAM secret key for DOCS_SCW_SECRET_KEY in GitHub Actions"
  value       = var.enable_docs_hosting ? module.docs_site[0].docs_deploy_secret_key : null
  sensitive   = true
}

output "docs_https_note" {
  description = "TLS / Edge Services reminder for docs custom domain"
  value       = var.enable_docs_hosting ? module.docs_site[0].https_note : null
}

output "docs_dns_record" {
  description = "FQDN for docs subdomain when DNS module created the record"
  value       = var.enable_dns && var.enable_docs_hosting ? module.dns[0].docs_dns_record : null
}
