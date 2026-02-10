# VPC
resource "scaleway_vpc" "main" {
  name   = "${var.app_name}-vpc-${var.environment}"
  tags   = concat(var.tags, ["environment:${var.environment}"])
  region = var.region
}

# Private Network
resource "scaleway_vpc_private_network" "main" {
  name   = "${var.app_name}-private-${var.environment}"
  vpc_id = scaleway_vpc.main.id
  region = var.region
  tags   = concat(var.tags, ["environment:${var.environment}"])

  ipv4_subnet {
    subnet = var.private_network_cidr
  }
}

# Public Gateway for outbound internet access
resource "scaleway_vpc_public_gateway" "main" {
  count = var.enable_public_gateway ? 1 : 0

  name            = "${var.app_name}-gateway-${var.environment}"
  type            = var.public_gateway_type
  zone            = var.zone
  bastion_enabled = var.enable_bastion
  tags            = concat(var.tags, ["environment:${var.environment}"])
}

# Public Gateway IP
resource "scaleway_vpc_public_gateway_ip" "main" {
  count = var.enable_public_gateway ? 1 : 0

  zone = var.zone
  tags = concat(var.tags, ["environment:${var.environment}"])
}

# Connect gateway to IP (reverse DNS)
resource "scaleway_vpc_public_gateway_ip_reverse_dns" "main" {
  count = var.enable_public_gateway && var.reverse_dns_hostname != null ? 1 : 0

  gateway_ip_id = scaleway_vpc_public_gateway_ip.main[0].id
  reverse       = var.reverse_dns_hostname
  zone          = var.zone
}

# DHCP configuration for private network
resource "scaleway_vpc_public_gateway_dhcp" "main" {
  count = var.enable_public_gateway ? 1 : 0

  subnet = var.private_network_cidr
  zone   = var.zone
}

# Connect private network to public gateway
resource "scaleway_vpc_gateway_network" "main" {
  count = var.enable_public_gateway ? 1 : 0

  gateway_id         = scaleway_vpc_public_gateway.main[0].id
  private_network_id = scaleway_vpc_private_network.main.id
  dhcp_id            = scaleway_vpc_public_gateway_dhcp.main[0].id
  enable_masquerade  = true
  zone               = var.zone
}

# Security Group for API instances
resource "scaleway_instance_security_group" "api" {
  name                    = "${var.app_name}-api-sg-${var.environment}"
  inbound_default_policy  = "drop"
  outbound_default_policy = "accept"
  zone                    = var.zone

  # Allow HTTPS from load balancer (internal, when LB is enabled)
  inbound_rule {
    action   = "accept"
    port     = 8443
    protocol = "TCP"
    ip_range = var.private_network_cidr
  }

  # Allow HTTP from public internet (for Caddy / Let's Encrypt ACME challenge)
  dynamic "inbound_rule" {
    for_each = var.enable_public_https ? [1] : []
    content {
      action   = "accept"
      port     = 80
      protocol = "TCP"
    }
  }

  # Allow HTTPS from public internet (for Caddy auto-SSL)
  dynamic "inbound_rule" {
    for_each = var.enable_public_https ? [1] : []
    content {
      action   = "accept"
      port     = 443
      protocol = "TCP"
    }
  }

  # Allow SSH (optional - for debugging)
  dynamic "inbound_rule" {
    for_each = var.enable_bastion ? var.ssh_allowed_cidrs : []
    content {
      action   = "accept"
      port     = 22
      protocol = "TCP"
      ip_range = inbound_rule.value
    }
  }

  tags = concat(var.tags, ["environment:${var.environment}"])
}

# Security Group for Worker instances
resource "scaleway_instance_security_group" "worker" {
  name                    = "${var.app_name}-worker-sg-${var.environment}"
  inbound_default_policy  = "drop"
  outbound_default_policy = "accept"
  zone                    = var.zone

  # Allow SSH (optional - for debugging)
  dynamic "inbound_rule" {
    for_each = var.enable_bastion ? [1] : []
    content {
      action   = "accept"
      port     = 22
      protocol = "TCP"
    }
  }

  tags = concat(var.tags, ["environment:${var.environment}"])
}

# Security Group for Redis (self-managed)
resource "scaleway_instance_security_group" "redis" {
  name                    = "${var.app_name}-redis-sg-${var.environment}"
  inbound_default_policy  = "drop"
  outbound_default_policy = "accept"
  zone                    = var.zone

  # Allow Redis port from private network
  inbound_rule {
    action   = "accept"
    port     = 6379
    protocol = "TCP"
    ip_range = var.private_network_cidr
  }

  tags = concat(var.tags, ["environment:${var.environment}"])
}

# Load Balancer
resource "scaleway_lb" "api" {
  count = var.enable_load_balancer ? 1 : 0

  name                    = "${var.app_name}-lb-${var.environment}"
  ip_id                   = scaleway_lb_ip.api[0].id
  type                    = var.lb_type
  zone                    = var.zone
  ssl_compatibility_level = "ssl_compatibility_level_modern"

  private_network {
    private_network_id = scaleway_vpc_private_network.main.id
    dhcp_config        = true
  }

  tags = concat(var.tags, ["environment:${var.environment}"])
}

# Load Balancer IP
resource "scaleway_lb_ip" "api" {
  count = var.enable_load_balancer ? 1 : 0

  zone = var.zone
}

# Load Balancer Backend
resource "scaleway_lb_backend" "api" {
  count = var.enable_load_balancer ? 1 : 0

  lb_id            = scaleway_lb.api[0].id
  name             = "${var.app_name}-backend-${var.environment}"
  forward_protocol = "http"
  ssl_bridging     = true
  forward_port     = 8443
  server_ips       = var.backend_server_ips

  health_check_http {
    uri    = var.health_check_path
    method = "GET"
    code   = 200
  }

  health_check_timeout  = "5s"
  health_check_delay    = "10s"
  health_check_max_retries = 3

  sticky_sessions             = "cookie"
  sticky_sessions_cookie_name = "ciel_session"
}

# Load Balancer Frontend - HTTP (redirects to HTTPS via ACL)
resource "scaleway_lb_frontend" "http" {
  count = var.enable_load_balancer ? 1 : 0

  lb_id        = scaleway_lb.api[0].id
  backend_id   = scaleway_lb_backend.api[0].id
  name         = "${var.app_name}-http-${var.environment}"
  inbound_port = 80
}

resource "scaleway_lb_acl" "http_to_https_redirect" {
  count = var.enable_load_balancer && length(var.ssl_certificate_ids) > 0 ? 1 : 0

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

# Load Balancer Frontend - HTTPS
resource "scaleway_lb_frontend" "https" {
  count = var.enable_load_balancer && length(var.ssl_certificate_ids) > 0 ? 1 : 0

  lb_id           = scaleway_lb.api[0].id
  backend_id      = scaleway_lb_backend.api[0].id
  name            = "${var.app_name}-https-${var.environment}"
  inbound_port    = 443
  certificate_ids = var.ssl_certificate_ids
}

locals {
  lb_frontend_id = var.enable_load_balancer ? (
    length(scaleway_lb_frontend.https) > 0 ? scaleway_lb_frontend.https[0].id : scaleway_lb_frontend.http[0].id
  ) : null
}

resource "scaleway_lb_acl" "blocked_ips" {
  count = var.enable_load_balancer && var.enable_basic_waf ? length(var.blocked_ip_ranges) : 0

  frontend_id = local.lb_frontend_id
  name        = "${var.app_name}-block-${count.index}-${var.environment}"
  index       = count.index + 1

  action {
    type = "deny"
  }

  match {
    ip_subnet = [var.blocked_ip_ranges[count.index]]
  }
}

# Bastion host (optional)
resource "scaleway_instance_server" "bastion" {
  count = var.enable_bastion ? 1 : 0

  name  = "${var.app_name}-bastion-${var.environment}"
  type  = var.bastion_instance_type
  image = "debian_bookworm"
  zone  = var.zone

  private_network {
    pn_id = scaleway_vpc_private_network.main.id
  }

  tags = concat(var.tags, ["environment:${var.environment}", "role:bastion"])
}
