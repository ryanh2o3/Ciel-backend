# Dev Environment Configuration
# Single instance (API + Redis) + Serverless Container for media processing

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
  environment = "dev"
  tags        = ["environment:dev", "managed-by:terraform"]

  # Construct DATABASE_URL for the serverless worker container
  serverless_database_url = "postgres://${module.database.database_user}:${var.db_user_password}@${module.database.private_endpoint}:${module.database.endpoint_port}/${module.database.database_name}?sslmode=require"
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

  # Dev-specific settings
  enable_load_balancer  = false  # No LB needed for dev
  enable_bastion        = true   # Allow SSH access for debugging
  enable_public_gateway = true
  enable_public_https   = true   # Caddy handles SSL directly on the instance
  private_network_cidr  = "10.0.1.0/24"
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

  # Dev-specific settings - smallest database
  db_node_type       = "DB-DEV-S"
  enable_ha          = false
  volume_size_in_gb  = 5
  read_replica_count = 0

  # Database credentials
  db_admin_password  = var.db_admin_password
  db_user_password   = var.db_user_password

  # Network
  private_network_id = module.networking.private_network_id
}

# Cache Module — disabled in combined mode (Redis runs on the compute instance)
module "cache" {
  source = "../../modules/cache"

  enabled            = false  # Redis is embedded in the combined instance

  project_id         = var.project_id
  region             = var.region
  zone               = var.zone
  environment        = local.environment
  app_name           = local.app_name
  tags               = local.tags

  use_managed_redis   = false
  redis_password      = var.redis_password

  # Network dependencies (not used when disabled, but required by module)
  private_network_id = module.networking.private_network_id
  security_group_id  = module.networking.redis_security_group_id
}

# Storage Module
module "storage" {
  source = "../../modules/storage"

  project_id           = var.project_id
  region               = var.region
  environment          = local.environment
  app_name             = local.app_name
  tags                 = local.tags

  # Dev-specific settings
  cors_allowed_origins = ["https://ciel-social.eu", "https://api.ciel-social.eu"]
  enable_cdn           = false  # No CDN for dev
  enable_glacier_transition = false
}

# Messaging Module
module "messaging" {
  source = "../../modules/messaging"

  project_id                = var.project_id
  region                    = var.region
  environment               = local.environment
  app_name                  = local.app_name
  tags                      = local.tags

  # Dev-specific settings
  enable_dlq                = true
  message_retention_seconds = 1209600  # 14 days for dev
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

# Compute Module — combined mode + serverless worker
module "compute" {
  source = "../../modules/compute"

  project_id               = var.project_id
  region                   = var.region
  zone                     = var.zone
  environment              = local.environment
  app_name                 = local.app_name
  tags                     = local.tags

  # Combined mode: API + Redis on one DEV1-M instance
  enable_combined_mode     = true
  combined_instance_type   = "DEV1-M"
  embedded_redis_maxmemory_mb = 512
  api_domain               = "api.${var.domain_name}"

  # No separate API/worker instances
  api_instance_count       = 0
  worker_instance_count    = 0

  # Serverless Container for media processing
  enable_serverless_worker    = true
  serverless_worker_cpu       = 1000  # 1 vCPU
  serverless_worker_memory    = 512   # 512 MB
  serverless_worker_min_scale = 0     # Scale to zero
  serverless_worker_max_scale = 3
  serverless_database_url     = local.serverless_database_url
  serverless_s3_access_key    = module.storage.s3_access_key
  serverless_s3_secret_key    = module.storage.s3_secret_key

  # Container image
  container_image_tag      = var.container_image_tag

  # Network dependencies
  private_network_id       = module.networking.private_network_id
  api_security_group_id    = module.networking.api_security_group_id
  worker_security_group_id = module.networking.worker_security_group_id
  load_balancer_backend_id = null  # No LB in dev

  # Application configuration from other modules
  db_host                  = module.database.private_endpoint
  db_port                  = module.database.endpoint_port
  db_name                  = module.database.database_name
  db_user                  = module.database.database_user
  db_password_secret_id    = module.secrets.db_password_secret_id
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
  rust_log                 = "debug,tower_http=debug"
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

  # Dev-specific settings
  enable_alerts = false  # No alerts for dev
}

# PAT rules — route internet traffic through the public gateway to the combined instance
resource "scaleway_vpc_public_gateway_pat_rule" "http" {
  gateway_id   = module.networking.public_gateway_id
  private_ip   = module.compute.api_instance_ips[0]
  private_port = 80
  public_port  = 80
  protocol     = "tcp"
  zone         = var.zone
}

resource "scaleway_vpc_public_gateway_pat_rule" "https" {
  gateway_id   = module.networking.public_gateway_id
  private_ip   = module.compute.api_instance_ips[0]
  private_port = 443
  public_port  = 443
  protocol     = "tcp"
  zone         = var.zone
}

# DNS Module — points api.ciel-social.eu at the public gateway
module "dns" {
  source = "../../modules/dns"

  count = var.enable_dns ? 1 : 0

  domain_name      = var.domain_name
  api_subdomain    = "api"
  cdn_subdomain    = "dev-media"
  load_balancer_ip = module.networking.public_gateway_ip  # Gateway public IP
  cdn_endpoint     = module.storage.cdn_endpoint

  # Dev-specific settings
  enable_api_dns  = true
  enable_cdn_dns  = false
  enable_www_dns  = false
  enable_root_dns = false
  enable_ssl      = false  # Caddy handles SSL on the instance
}
