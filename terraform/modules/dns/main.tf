# DNS Module for Scaleway
# Manages domain records and SSL certificates

# Domain records for API
resource "scaleway_domain_record" "api" {
  count = var.enable_api_dns ? 1 : 0

  dns_zone = var.domain_name
  name     = var.api_subdomain
  type     = "A"
  data     = var.load_balancer_ip
  ttl      = 300
  priority = null
}

# Domain records for CDN/media
resource "scaleway_domain_record" "cdn" {
  count = var.enable_cdn_dns ? 1 : 0

  dns_zone = var.domain_name
  name     = var.cdn_subdomain
  type     = "CNAME"
  data     = var.cdn_endpoint
  ttl      = 300
  priority = null
}

# Domain records for main website
resource "scaleway_domain_record" "www" {
  count = var.enable_www_dns ? 1 : 0

  dns_zone = var.domain_name
  name     = "www"
  type     = "A"
  data     = var.load_balancer_ip
  ttl      = 300
  priority = null
}

# Domain records for root domain
resource "scaleway_domain_record" "root" {
  count = var.enable_root_dns ? 1 : 0

  dns_zone = var.domain_name
  name     = "@"
  type     = "A"
  data     = var.load_balancer_ip
  ttl      = 300
  priority = null
}

# Build the list of domains that actually resolve to the LB
locals {
  # The common_name must be a domain that resolves to the LB.
  # Prefer the API subdomain (always present when SSL is enabled).
  ssl_common_name = "${var.api_subdomain}.${var.domain_name}"

  ssl_sans = concat(
    var.enable_root_dns ? [var.domain_name] : [],
    var.enable_www_dns ? ["www.${var.domain_name}"] : [],
    var.enable_cdn_dns ? ["${var.cdn_subdomain}.${var.domain_name}"] : [],
  )
}

# SSL Certificate via Let's Encrypt (managed by Load Balancer)
resource "scaleway_lb_certificate" "ssl" {
  count = var.enable_ssl ? 1 : 0

  lb_id = var.load_balancer_id
  name  = "${var.domain_name}-cert"

  letsencrypt {
    common_name              = local.ssl_common_name
    subject_alternative_name = local.ssl_sans
  }

  # DNS records must resolve to the LB IP before Let's Encrypt validation
  depends_on = [
    scaleway_domain_record.api,
    scaleway_domain_record.www,
    scaleway_domain_record.root,
    scaleway_domain_record.cdn,
  ]

  lifecycle {
    create_before_destroy = true
  }
}
