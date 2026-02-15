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
  ssl_certificate_ids   = var.enable_dns ? module.dns[0].ssl_certificate_ids : []
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
  container_image_tag      = var.container_image_tag

  # Network dependencies
  private_network_id       = module.networking.private_network_id
  api_security_group_id    = module.networking.api_security_group_id
  worker_security_group_id = module.networking.worker_security_group_id
  load_balancer_backend_id = module.networking.load_balancer_backend_id
  
  # Application configuration from other modules
  db_host                  = module.database.private_endpoint
  db_port                  = module.database.endpoint_port
  db_name                  = module.database.database_name
  db_user                  = module.database.database_user
  db_password_secret_id    = module.secrets.db_password_secret_id
  redis_host               = module.cache.redis_host
  redis_port               = module.cache.redis_port
  redis_use_tls            = module.cache.redis_use_tls
  redis_password_secret_id = module.secrets.redis_password_secret_id
  s3_endpoint              = module.storage.s3_endpoint
  s3_region                = var.region
  s3_bucket                = module.storage.bucket_name
  s3_public_endpoint       = module.storage.s3_public_endpoint
  s3_access_key_secret_id  = module.secrets.s3_access_key_secret_id
  s3_secret_key_secret_id  = module.secrets.s3_secret_key_secret_id
  queue_endpoint           = module.messaging.queue_endpoint
  queue_region             = var.region
  queue_name               = module.messaging.queue_name
  sqs_access_key_secret_id = module.secrets.sqs_access_key_secret_id
  sqs_secret_key_secret_id = module.secrets.sqs_secret_key_secret_id
  paseto_access_key_secret_id  = module.secrets.paseto_access_key_secret_id
  paseto_refresh_key_secret_id = module.secrets.paseto_refresh_key_secret_id
  admin_token_secret_id        = module.secrets.admin_token_secret_id != null ? module.secrets.admin_token_secret_id : ""
  rust_log                 = "info"
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
