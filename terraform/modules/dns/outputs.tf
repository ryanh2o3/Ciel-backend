# DNS Module Outputs

output "api_dns_record" {
  description = "DNS record for API"
  value       = var.enable_api_dns ? scaleway_domain_record.api[0].fqdn : null
}

output "cdn_dns_record" {
  description = "DNS record for CDN"
  value       = var.enable_cdn_dns ? scaleway_domain_record.cdn[0].fqdn : null
}

output "www_dns_record" {
  description = "DNS record for www"
  value       = var.enable_www_dns ? scaleway_domain_record.www[0].fqdn : null
}

output "root_dns_record" {
  description = "DNS record for root domain"
  value       = var.enable_root_dns ? scaleway_domain_record.root[0].fqdn : null
}

output "ssl_certificate_id" {
  description = "SSL certificate ID"
  value       = var.enable_ssl ? scaleway_lb_certificate.ssl[0].id : null
}

output "ssl_certificate_ids" {
  description = "List of SSL certificate IDs for load balancer"
  value       = var.enable_ssl ? [scaleway_lb_certificate.ssl[0].id] : []
}