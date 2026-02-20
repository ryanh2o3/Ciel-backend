# Production Environment Configuration
# This configuration uses production-grade resources with high availability

terraform {
  required_version = ">= 1.5.0"

  required_providers {
    scaleway = {
      source  = "scaleway/scaleway"
      version = "~> 2.40"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.6"
    }
  }
}

# Configure the Scaleway provider
provider "scaleway" {
  zone            = var.zone
  region          = var.region
  project_id      = var.project_id
  organization_id = var.organization_id
}

# Shared variables
locals {
  app_name    = "ciel"
  environment = "prod"
  tags        = ["environment:prod", "managed-by:terraform"]
}

# Networking Module
module "networking" {
  source = "../../modules/networking"

  project_id           = var.project_id
  region               = var.region
  zone                 = var.zone
  environment          = local.environment
  app_name             = local.app_name
  tags                 = local.tags

  # Production-specific settings
  enable_load_balancer  = true
  enable_bastion        = false  # No bastion in production for security
  enable_public_gateway = true
  private_network_cidr  = "10.0.3.0/24"
  lb_type               = "LB-GP-S"
  ssh_allowed_cidrs     = var.ssh_allowed_cidrs
}

# Database Module
module "database" {
  source = "../../modules/database"

  project_id         = var.project_id
  region             = var.region
  zone               = var.zone
  environment        = local.environment
  app_name           = local.app_name
  tags               = local.tags

  # Production-specific settings - HA database
  db_node_type       = "DB-PRO2-XXS"
  enable_ha          = true
  volume_size_in_gb  = 50
  read_replica_count = 1

  # Database credentials
  db_admin_password  = var.db_admin_password
  db_user_password   = var.db_user_password

  # Network
  private_network_id = module.networking.private_network_id
}

# Cache Module
module "cache" {
  source = "../../modules/cache"

  project_id              = var.project_id
  region                  = var.region
  zone                    = var.zone
  environment             = local.environment
  app_name                = local.app_name
  tags                    = local.tags

  # Production-specific settings - managed Redis for reliability
  use_managed_redis       = true
  managed_redis_node_type = "RED1-micro"
  redis_password          = var.redis_password

  # Network dependencies
  private_network_id      = module.networking.private_network_id
  security_group_id       = module.networking.redis_security_group_id
}

# Storage Module
module "storage" {
  source = "../../modules/storage"

  project_id               = var.project_id
  region                   = var.region
  environment              = local.environment
  app_name                 = local.app_name
  tags                     = local.tags

  # Production-specific settings
  cors_allowed_origins     = ["https://ciel-social.eu", "https://www.ciel-social.eu"]
  enable_cdn               = true
  enable_glacier_transition = true
}

# Messaging Module
module "messaging" {
  source = "../../modules/messaging"

  project_id                = var.project_id
  region                    = var.region
  zone                      = var.zone
  environment               = local.environment
  app_name                  = local.app_name
  tags                      = local.tags

  # Production-specific settings
  enable_dlq                = true
  message_retention_seconds = 345600  # 4 days
}

# Secrets Module
module "secrets" {
  source = "../../modules/secrets"

  project_id         = var.project_id
  region             = var.region
  zone               = var.zone
  environment        = local.environment
  app_name           = local.app_name
  tags               = local.tags

  # Secrets from variables
  paseto_access_key  = var.paseto_access_key
  paseto_refresh_key = var.paseto_refresh_key
  admin_token        = var.admin_token
  generate_db_password   = false
  generate_redis_password = false
  db_password        = var.db_user_password
  redis_password     = var.redis_password
  s3_access_key      = module.storage.s3_access_key
  s3_secret_key      = module.storage.s3_secret_key
  sqs_access_key     = module.messaging.sqs_access_key
  sqs_secret_key     = module.messaging.sqs_secret_key
}

# Compute Module
module "compute" {
  source = "../../modules/compute"

  project_id               = var.project_id
  region                   = var.region
  zone                     = var.zone
  environment              = local.environment
  app_name                 = local.app_name
  tags                     = local.tags

  # Production-specific settings - multiple instances for HA
  api_instance_count       = 2
  api_instance_type        = "GP1-M"
  worker_instance_count    = 2
  worker_instance_type     = "GP1-S"

  # Use production container image with version tag
  container_image          = "rg.fr-par.scw.cloud/ciel-social/ciel-backend"
  container_image_tag      = var.container_image_tag

  # Network dependencies
  private_network_id       = module.networking.private_network_id
  api_security_group_id    = module.networking.api_security_group_id
  worker_security_group_id = module.networking.worker_security_group_id

  # Application configuration from other modules
  db_host                = module.database.private_endpoint
  db_port                = module.database.endpoint_port
  db_name                = module.database.database_name
  db_user                = module.database.database_user
  db_password            = var.db_user_password
  migration_database_url = module.database.migration_database_url
  redis_host         = module.cache.redis_host
  redis_port         = module.cache.redis_port
  redis_use_tls      = module.cache.redis_use_tls
  redis_password     = var.redis_password
  s3_endpoint        = module.storage.s3_endpoint
  s3_region          = var.region
  s3_bucket          = module.storage.bucket_name
  s3_public_endpoint = module.storage.s3_public_endpoint
  s3_access_key      = module.storage.s3_access_key
  s3_secret_key      = module.storage.s3_secret_key
  queue_endpoint     = module.messaging.queue_endpoint
  queue_region       = var.region
  queue_name         = module.messaging.queue_name
  sqs_access_key     = module.messaging.sqs_access_key
  sqs_secret_key     = module.messaging.sqs_secret_key
  paseto_access_key  = var.paseto_access_key
  paseto_refresh_key = var.paseto_refresh_key
  admin_token        = var.admin_token
  rust_log           = "info"
}

# Observability Module
module "observability" {
  source = "../../modules/observability"

  project_id    = var.project_id
  region        = var.region
  zone          = var.zone
  environment   = local.environment
  app_name      = local.app_name
  tags          = local.tags

  # Production-specific settings
  enable_alerts       = true
  alert_contact_emails = var.alert_contact_emails
}

# DNS Module
module "dns" {
  source = "../../modules/dns"

  count = var.enable_dns ? 1 : 0

  domain_name      = var.domain_name
  api_subdomain    = "api"
  cdn_subdomain    = "media"
  load_balancer_ip = module.networking.load_balancer_ip
  load_balancer_id = module.networking.load_balancer_id
  cdn_endpoint     = module.storage.cdn_endpoint

  # Production-specific settings
  enable_api_dns  = true
  enable_cdn_dns  = true
  enable_www_dns  = true
  enable_root_dns = true
  enable_ssl      = true
}

# ============================================================
# Load Balancer Backend / Frontends / ACLs
# Defined here (not in networking module) to avoid circular
# dependency: networking creates the LB, compute creates
# instances, and the root module wires them together.
# ============================================================

resource "scaleway_lb_backend" "api" {
  count = var.enable_dns ? 1 : 0

  lb_id            = module.networking.load_balancer_id
  name             = "ciel-backend-prod"
  forward_protocol = "http"
  ssl_bridging     = true
  forward_port     = 8443
  server_ips       = module.compute.api_instance_ips

  ignore_ssl_server_verify = true  # Backend uses self-signed TLS cert

  health_check_tcp {}

  health_check_timeout     = "5s"
  health_check_delay       = "10s"
  health_check_max_retries = 3

  sticky_sessions             = "cookie"
  sticky_sessions_cookie_name = "ciel_session"
}

resource "scaleway_lb_frontend" "http" {
  count = var.enable_dns ? 1 : 0

  lb_id        = module.networking.load_balancer_id
  backend_id   = scaleway_lb_backend.api[0].id
  name         = "ciel-http-prod"
  inbound_port = 80
}

resource "scaleway_lb_acl" "http_to_https_redirect" {
  count = var.enable_dns ? 1 : 0

  frontend_id = scaleway_lb_frontend.http[0].id
  name        = "ciel-http-redirect-prod"
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

resource "scaleway_lb_frontend" "https" {
  count = var.enable_dns ? 1 : 0

  lb_id           = module.networking.load_balancer_id
  backend_id      = scaleway_lb_backend.api[0].id
  name            = "ciel-https-prod"
  inbound_port    = 443
  certificate_ids = module.dns[0].ssl_certificate_ids
}

# API Security Module (Optimized for Mobile Apps)
module "api_security" {
  source = "../../modules/api_security"

  project_id             = var.project_id
  region                 = var.region
  zone                   = var.zone
  environment            = local.environment
  app_name               = local.app_name
  tags                   = local.tags
  private_network_id     = module.networking.private_network_id
  ssl_certificate_ids    = var.enable_dns ? module.dns[0].ssl_certificate_ids : []

  # API Gateway configuration
  enable_api_gateway     = false  # Using main LB, not a separate gateway

  # Security configuration
  enable_ip_restrictions = var.enable_ip_restrictions
  allowed_ips            = var.allowed_ips
  api_keys               = var.api_keys
}
