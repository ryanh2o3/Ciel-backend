# Container Registry
resource "scaleway_registry_namespace" "main" {
  name        = "${var.app_name}-${var.environment}"
  description = "Container registry for ${var.app_name} ${var.environment}"
  is_public   = var.registry_is_public
  region      = var.region
}

# IAM application for runtime access (registry pull + secrets read)
resource "scaleway_iam_application" "runtime" {
  name        = "${var.app_name}-${var.environment}-${var.runtime_iam_application_name}"
  description = "Runtime access for ${var.app_name} ${var.environment} instances"
}

resource "scaleway_iam_policy" "runtime" {
  name           = "${var.app_name}-runtime-policy-${var.environment}"
  description    = "Registry pull and secrets read for ${var.app_name} ${var.environment}"
  application_id = scaleway_iam_application.runtime.id

  rule {
    project_ids = [var.project_id]
    permission_set_names = [
      "ContainerRegistryReadOnly",
      "SecretManagerReadOnly",
    ]
  }
}

resource "scaleway_iam_api_key" "runtime" {
  application_id = scaleway_iam_application.runtime.id
  description    = "Runtime API key for ${var.app_name} ${var.environment}"
}

# ============================================================
# Cloud-init templates
# ============================================================

# Standard API-only cloud-init (multi-instance mode)
locals {
  cloud_init_api = !var.enable_combined_mode ? templatefile("${path.module}/cloud-init-api.yaml", {
    app_name                   = var.app_name
    image_tag                  = var.container_image_tag
    registry_endpoint          = scaleway_registry_namespace.main.endpoint
    scw_access_key             = scaleway_iam_api_key.runtime.access_key
    scw_secret_key             = scaleway_iam_api_key.runtime.secret_key
    scw_region                 = var.region
    scw_zone                   = var.zone
    http_addr                  = var.http_addr
    db_host                    = var.db_host
    db_port                    = var.db_port
    db_name                    = var.db_name
    db_user                    = var.db_user
    db_password_secret_id      = var.db_password_secret_id
    redis_host                 = var.redis_host
    redis_port                 = var.redis_port
    redis_use_tls              = var.redis_use_tls
    redis_password_secret_id   = var.redis_password_secret_id
    s3_endpoint                = var.s3_endpoint
    s3_region                  = var.s3_region
    s3_bucket                  = var.s3_bucket
    s3_public_endpoint         = var.s3_public_endpoint
    s3_access_key_secret_id    = var.s3_access_key_secret_id
    s3_secret_key_secret_id    = var.s3_secret_key_secret_id
    queue_endpoint             = var.queue_endpoint
    queue_region               = var.queue_region
    queue_name                 = var.queue_name
    sqs_access_key_secret_id   = var.sqs_access_key_secret_id
    sqs_secret_key_secret_id   = var.sqs_secret_key_secret_id
    paseto_access_key_secret_id  = var.paseto_access_key_secret_id
    paseto_refresh_key_secret_id = var.paseto_refresh_key_secret_id
    admin_token_secret_id        = var.admin_token_secret_id
    rust_log                   = var.rust_log
  }) : ""
}

# Combined cloud-init: API + Redis on one instance
locals {
  cloud_init_combined = var.enable_combined_mode ? templatefile("${path.module}/cloud-init-combined.yaml", {
    app_name                   = var.app_name
    image_tag                  = var.container_image_tag
    registry_endpoint          = scaleway_registry_namespace.main.endpoint
    scw_access_key             = scaleway_iam_api_key.runtime.access_key
    scw_secret_key             = scaleway_iam_api_key.runtime.secret_key
    scw_region                 = var.region
    scw_zone                   = var.zone
    http_addr                  = var.http_addr
    api_domain                 = var.api_domain
    db_host                    = var.db_host
    db_port                    = var.db_port
    db_name                    = var.db_name
    db_user                    = var.db_user
    db_password_secret_id      = var.db_password_secret_id
    redis_password_secret_id   = var.redis_password_secret_id
    redis_maxmemory_mb         = var.embedded_redis_maxmemory_mb
    s3_endpoint                = var.s3_endpoint
    s3_region                  = var.s3_region
    s3_bucket                  = var.s3_bucket
    s3_public_endpoint         = var.s3_public_endpoint
    s3_access_key_secret_id    = var.s3_access_key_secret_id
    s3_secret_key_secret_id    = var.s3_secret_key_secret_id
    queue_endpoint             = var.queue_endpoint
    queue_region               = var.queue_region
    queue_name                 = var.queue_name
    sqs_access_key_secret_id   = var.sqs_access_key_secret_id
    sqs_secret_key_secret_id   = var.sqs_secret_key_secret_id
    paseto_access_key_secret_id  = var.paseto_access_key_secret_id
    paseto_refresh_key_secret_id = var.paseto_refresh_key_secret_id
    admin_token_secret_id        = var.admin_token_secret_id
    rust_log                   = var.rust_log
  }) : ""
}

# Cloud-init template for Worker instances (legacy polling mode)
locals {
  cloud_init_worker = var.worker_instance_count > 0 ? templatefile("${path.module}/cloud-init-worker.yaml", {
    app_name                   = var.app_name
    image_tag                  = var.container_image_tag
    registry_endpoint          = scaleway_registry_namespace.main.endpoint
    scw_access_key             = scaleway_iam_api_key.runtime.access_key
    scw_secret_key             = scaleway_iam_api_key.runtime.secret_key
    scw_region                 = var.region
    scw_zone                   = var.zone
    db_host                    = var.db_host
    db_port                    = var.db_port
    db_name                    = var.db_name
    db_user                    = var.db_user
    db_password_secret_id      = var.db_password_secret_id
    redis_host                 = var.redis_host
    redis_port                 = var.redis_port
    redis_use_tls              = var.redis_use_tls
    redis_password_secret_id   = var.redis_password_secret_id
    s3_endpoint                = var.s3_endpoint
    s3_region                  = var.s3_region
    s3_bucket                  = var.s3_bucket
    s3_public_endpoint         = var.s3_public_endpoint
    s3_access_key_secret_id    = var.s3_access_key_secret_id
    s3_secret_key_secret_id    = var.s3_secret_key_secret_id
    queue_endpoint             = var.queue_endpoint
    queue_region               = var.queue_region
    queue_name                 = var.queue_name
    sqs_access_key_secret_id   = var.sqs_access_key_secret_id
    sqs_secret_key_secret_id   = var.sqs_secret_key_secret_id
    rust_log                   = var.rust_log
  }) : ""
}

# ============================================================
# Compute Instances
# ============================================================

# API Instances (standard multi-instance mode)
resource "scaleway_instance_server" "api" {
  count = var.enable_combined_mode ? 0 : var.api_instance_count

  name  = "${var.app_name}-api-${var.environment}-${count.index + 1}"
  type  = var.api_instance_type
  image = "debian_bookworm"
  zone  = var.zone

  security_group_id = var.api_security_group_id

  private_network {
    pn_id = var.private_network_id
  }

  user_data = {
    cloud-init = local.cloud_init_api
  }

  tags = concat(var.tags, [
    "environment:${var.environment}",
    "role:api",
    "app:${var.app_name}"
  ])

  lifecycle {
    create_before_destroy = true
  }
}

# Combined Instance (API + Redis on one box)
# Flexible IP for combined instance (direct public access)
resource "scaleway_instance_ip" "combined" {
  count = var.enable_combined_mode ? 1 : 0
  zone  = var.zone
}

resource "scaleway_instance_server" "combined" {
  count = var.enable_combined_mode ? 1 : 0

  name  = "${var.app_name}-combined-${var.environment}"
  type  = var.combined_instance_type
  image = "debian_bookworm"
  zone  = var.zone
  ip_id = scaleway_instance_ip.combined[0].id

  security_group_id = var.api_security_group_id

  private_network {
    pn_id = var.private_network_id
  }

  user_data = {
    cloud-init = local.cloud_init_combined
  }

  tags = concat(var.tags, [
    "environment:${var.environment}",
    "role:combined",
    "app:${var.app_name}"
  ])

  lifecycle {
    create_before_destroy = true
  }
}

# Worker Instances (legacy polling mode — set count=0 to disable)
resource "scaleway_instance_server" "worker" {
  count = var.worker_instance_count

  name  = "${var.app_name}-worker-${var.environment}-${count.index + 1}"
  type  = var.worker_instance_type
  image = "debian_bookworm"
  zone  = var.zone

  security_group_id = var.worker_security_group_id

  private_network {
    pn_id = var.private_network_id
  }

  user_data = {
    cloud-init = local.cloud_init_worker
  }

  tags = concat(var.tags, [
    "environment:${var.environment}",
    "role:worker",
    "app:${var.app_name}"
  ])

  lifecycle {
    create_before_destroy = true
  }
}

# ============================================================
# Serverless Container (event-driven media worker)
# ============================================================

resource "scaleway_container_namespace" "worker" {
  count = var.enable_serverless_worker ? 1 : 0

  name        = "${var.app_name}-worker-${var.environment}"
  description = "Serverless media worker for ${var.app_name} ${var.environment}"
  region      = var.region
}

resource "scaleway_container" "media_processor" {
  count = var.enable_serverless_worker ? 1 : 0

  name           = "media-processor"
  namespace_id   = scaleway_container_namespace.worker[0].id
  registry_image = "${scaleway_registry_namespace.main.endpoint}/${var.app_name}:${var.container_image_tag}"
  port           = 8080
  cpu_limit      = var.serverless_worker_cpu
  memory_limit   = var.serverless_worker_memory
  min_scale      = var.serverless_worker_min_scale
  max_scale      = var.serverless_worker_max_scale
  timeout        = var.serverless_worker_timeout
  deploy         = true

  environment_variables = {
    APP_MODE    = "serverless-worker"
    HTTP_ADDR   = "0.0.0.0:8080"
    S3_ENDPOINT = var.s3_endpoint
    S3_REGION   = var.s3_region
    S3_BUCKET   = var.s3_bucket
    RUST_LOG    = var.rust_log
  }

  secret_environment_variables = {
    DATABASE_URL          = var.serverless_database_url
    AWS_ACCESS_KEY_ID     = var.serverless_s3_access_key
    AWS_SECRET_ACCESS_KEY = var.serverless_s3_secret_key
  }
}

resource "scaleway_container_trigger" "sqs_media" {
  count = var.enable_serverless_worker ? 1 : 0

  container_id = scaleway_container.media_processor[0].id
  name         = "media-jobs-trigger"

  sqs {
    project_id = var.project_id
    queue      = var.queue_name
    region     = var.queue_region
  }
}
