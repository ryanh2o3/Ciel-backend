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

# Connect private network to public gateway
resource "scaleway_vpc_gateway_network" "main" {
  count = var.enable_public_gateway ? 1 : 0

  gateway_id         = scaleway_vpc_public_gateway.main[0].id
  private_network_id = scaleway_vpc_private_network.main.id
  enable_masquerade  = true
  zone               = var.zone

  ipam_config {
    push_default_route = true
  }
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

  # Allow HTTP from load balancer (internal, for combined/dev mode without SSL bridging)
  inbound_rule {
    action   = "accept"
    port     = 8080
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
      ip_range = "0.0.0.0/0"
    }
  }

  # Allow HTTPS from public internet (for Caddy auto-SSL)
  dynamic "inbound_rule" {
    for_each = var.enable_public_https ? [1] : []
    content {
      action   = "accept"
      port     = 443
      protocol = "TCP"
      ip_range = "0.0.0.0/0"
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
