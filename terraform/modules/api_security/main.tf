# API Security Module
# Security configuration for API access control
# Note: Scaleway LB doesn't support WAF via Terraform - implement at application level

# API Gateway - Application Load Balancer
resource "scaleway_lb" "api_gateway" {
  count = var.enable_api_gateway ? 1 : 0

  name                    = "${var.app_name}-api-gateway-${var.environment}"
  ip_id                   = scaleway_lb_ip.api_gateway[0].id
  type                    = var.gateway_type
  zone                    = var.zone
  ssl_compatibility_level = "ssl_compatibility_level_modern"

  private_network {
    private_network_id = var.private_network_id
    dhcp_config        = true
  }

  tags = concat(var.tags, ["environment:${var.environment}", "role:api-gateway"])
}

# Basic L7 ACL protections (optional)
locals {
  api_gateway_frontend_id = var.enable_api_gateway ? (
    length(scaleway_lb_frontend.https) > 0 ? scaleway_lb_frontend.https[0].id : scaleway_lb_frontend.http[0].id
  ) : null
}

resource "scaleway_lb_acl" "blocked_ips" {
  count = var.enable_api_gateway && var.enable_basic_waf ? length(var.blocked_ip_ranges) : 0

  frontend_id = local.api_gateway_frontend_id
  name        = "${var.app_name}-block-${count.index}-${var.environment}"
  index       = count.index + 1

  action {
    type = "deny"
  }

  match {
    ip_subnet = [var.blocked_ip_ranges[count.index]]
  }
}

# API Gateway IP
resource "scaleway_lb_ip" "api_gateway" {
  count = var.enable_api_gateway ? 1 : 0
  zone  = var.zone
}

# API Gateway Backend
resource "scaleway_lb_backend" "api_gateway" {
  count = var.enable_api_gateway ? 1 : 0

  lb_id            = scaleway_lb.api_gateway[0].id
  name             = "${var.app_name}-gateway-backend-${var.environment}"
  forward_protocol = "http"
  ssl_bridging     = true
  forward_port     = 8443

  health_check_http {
    uri    = var.health_check_path
    method = "GET"
    code   = 200
  }

  health_check_timeout     = "5s"
  health_check_delay       = "10s"
  health_check_max_retries = 3

  sticky_sessions             = "cookie"
  sticky_sessions_cookie_name = "ciel_session"
}

# API Gateway Frontend - HTTP (redirects to HTTPS via ACL)
resource "scaleway_lb_frontend" "http" {
  count = var.enable_api_gateway ? 1 : 0

  lb_id        = scaleway_lb.api_gateway[0].id
  backend_id   = scaleway_lb_backend.api_gateway[0].id
  name         = "${var.app_name}-http-gateway-${var.environment}"
  inbound_port = 80
}

resource "scaleway_lb_acl" "http_to_https_redirect" {
  count = var.enable_api_gateway && length(var.ssl_certificate_ids) > 0 ? 1 : 0

  frontend_id = scaleway_lb_frontend.http[0].id
  name        = "${var.app_name}-http-redirect-${var.environment}"
  index       = 1

  action {
    type = "redirect"
    redirect {
      type   = "scheme"
      target = "https"
      code   = 301
    }
  }

  match {
    http_filter       = "path_begin"
    http_filter_value = ["*"]
  }
}

# API Gateway Frontend - HTTPS
resource "scaleway_lb_frontend" "https" {
  count = var.enable_api_gateway && length(var.ssl_certificate_ids) > 0 ? 1 : 0

  lb_id           = scaleway_lb.api_gateway[0].id
  backend_id      = scaleway_lb_backend.api_gateway[0].id
  name            = "${var.app_name}-https-gateway-${var.environment}"
  inbound_port    = 443
  certificate_ids = var.ssl_certificate_ids
}

# API Keys for application authentication (stored in Secret Manager)
resource "scaleway_secret" "api_keys" {
  count = length(var.api_keys)

  name        = "${var.app_name}-api-key-${count.index}-${var.environment}"
  description = "API key for application authentication"
  tags        = concat(var.tags, ["environment:${var.environment}", "type:api-key"])
}

resource "scaleway_secret_version" "api_keys" {
  count = length(var.api_keys)

  secret_id = scaleway_secret.api_keys[count.index].id
  data      = var.api_keys[count.index]
}

# IP Restrictions via Security Group (optional)
resource "scaleway_instance_security_group" "api_gateway" {
  count = var.enable_ip_restrictions ? 1 : 0

  name                    = "${var.app_name}-api-gateway-sg-${var.environment}"
  inbound_default_policy  = "drop"
  outbound_default_policy = "accept"
  zone                    = var.zone

  # Allow HTTPS from allowed IPs only
  dynamic "inbound_rule" {
    for_each = var.allowed_ips
    content {
      action   = "accept"
      port     = 443
      protocol = "TCP"
      ip       = inbound_rule.value
    }
  }

  # Allow HTTP for redirects
  dynamic "inbound_rule" {
    for_each = var.allowed_ips
    content {
      action   = "accept"
      port     = 80
      protocol = "TCP"
      ip       = inbound_rule.value
    }
  }

  tags = concat(var.tags, ["environment:${var.environment}", "role:api-gateway"])
}

# NOTE: WAF and rate limiting should be implemented at the application level
# The Scaleway Terraform provider doesn't support LB WAF rules.
# Consider using:
# - tower-governor or similar rate limiting middleware in Rust
# - Application-level request validation
# - Cloudflare or similar CDN/WAF service in front of the LB
