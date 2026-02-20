# Container Registry
resource "scaleway_registry_namespace" "main" {
  name        = "${var.app_name}-${var.environment}"
  description = "Container registry for ${var.app_name} ${var.environment}"
  is_public   = var.registry_is_public
  region      = var.region
}

# IAM application for runtime access (registry pull)
resource "scaleway_iam_application" "runtime" {
  name        = "${var.app_name}-${var.environment}-${var.runtime_iam_application_name}"
  description = "Runtime access for ${var.app_name} ${var.environment} instances"
}

resource "scaleway_iam_policy" "runtime" {
  name           = "${var.app_name}-runtime-policy-${var.environment}"
  description    = "Registry pull for ${var.app_name} ${var.environment}"
  application_id = scaleway_iam_application.runtime.id

  rule {
    project_ids = [var.project_id]
    permission_set_names = [
      "ContainerRegistryReadOnly",
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

# Pre-compute connection URLs (URL-encode passwords for safe embedding)
locals {
  database_url = "postgres://${var.db_user}:${urlencode(var.db_password)}@${var.db_host}:${var.db_port}/${var.db_name}?sslmode=require"
  redis_url    = "${var.redis_use_tls ? "rediss" : "redis"}://:${urlencode(var.redis_password)}@${var.redis_host}:${var.redis_port}"
  redis_url_combined = "redis://:${urlencode(var.redis_password)}@redis:6379"
}

# Standard API-only cloud-init (multi-instance mode)
locals {
  cloud_init_api = !var.enable_combined_mode ? templatefile("${path.module}/cloud-init-api.yaml", {
    container_image  = var.container_image
    image_tag        = var.container_image_tag
    scw_access_key   = scaleway_iam_api_key.runtime.access_key
    scw_secret_key   = scaleway_iam_api_key.runtime.secret_key
    scw_region       = var.region
    http_addr        = var.http_addr
    database_url     = local.database_url
    redis_url        = local.redis_url
    s3_endpoint      = var.s3_endpoint
    s3_region        = var.s3_region
    s3_bucket        = var.s3_bucket
    s3_public_endpoint = var.s3_public_endpoint
    s3_access_key    = var.s3_access_key
    s3_secret_key    = var.s3_secret_key
    queue_endpoint   = var.queue_endpoint
    queue_region     = var.queue_region
    queue_name       = var.queue_name
    sqs_access_key   = var.sqs_access_key
    sqs_secret_key   = var.sqs_secret_key
    paseto_access_key  = var.paseto_access_key
    paseto_refresh_key = var.paseto_refresh_key
    admin_token      = var.admin_token
    rust_log         = var.rust_log
  }) : ""
}

# Combined cloud-init: API + Redis on one instance
locals {
  cloud_init_combined = var.enable_combined_mode ? templatefile("${path.module}/cloud-init-combined.yaml", {
    container_image    = var.container_image
    image_tag          = var.container_image_tag
    scw_access_key     = scaleway_iam_api_key.runtime.access_key
    scw_secret_key     = scaleway_iam_api_key.runtime.secret_key
    scw_region         = var.region
    http_addr          = var.http_addr
    database_url       = local.database_url
    migration_database_url = var.migration_database_url
    db_user            = var.db_user
    redis_url          = local.redis_url_combined
    redis_password     = var.redis_password
    redis_maxmemory_mb = var.embedded_redis_maxmemory_mb
    s3_endpoint        = var.s3_endpoint
    s3_region          = var.s3_region
    s3_bucket          = var.s3_bucket
    s3_public_endpoint = var.s3_public_endpoint
    s3_access_key      = var.s3_access_key
    s3_secret_key      = var.s3_secret_key
    queue_endpoint     = var.queue_endpoint
    queue_region       = var.queue_region
    queue_name         = var.queue_name
    sqs_access_key     = var.sqs_access_key
    sqs_secret_key     = var.sqs_secret_key
    paseto_access_key  = var.paseto_access_key
    paseto_refresh_key = var.paseto_refresh_key
    admin_token        = var.admin_token
    rust_log           = var.rust_log
  }) : ""
}

# Cloud-init template for Worker instances (legacy polling mode)
locals {
  cloud_init_worker = var.worker_instance_count > 0 ? templatefile("${path.module}/cloud-init-worker.yaml", {
    container_image  = var.container_image
    image_tag        = var.container_image_tag
    scw_access_key   = scaleway_iam_api_key.runtime.access_key
    scw_secret_key   = scaleway_iam_api_key.runtime.secret_key
    scw_region       = var.region
    database_url     = local.database_url
    redis_url        = local.redis_url
    s3_endpoint      = var.s3_endpoint
    s3_region        = var.s3_region
    s3_bucket        = var.s3_bucket
    s3_public_endpoint = var.s3_public_endpoint
    s3_access_key    = var.s3_access_key
    s3_secret_key    = var.s3_secret_key
    queue_endpoint   = var.queue_endpoint
    queue_region     = var.queue_region
    queue_name       = var.queue_name
    sqs_access_key   = var.sqs_access_key
    sqs_secret_key   = var.sqs_secret_key
    rust_log         = var.rust_log
  }) : ""
}

# ============================================================
# Compute Instances
# ============================================================

# Replacement triggers — when cloud-init changes (including image tag),
# the corresponding instance is automatically recreated.
resource "terraform_data" "api_replacement" {
  count = var.enable_combined_mode ? 0 : var.api_instance_count
  input = sha256(local.cloud_init_api)
}

resource "terraform_data" "combined_replacement" {
  count = var.enable_combined_mode ? 1 : 0
  input = sha256(local.cloud_init_combined)
}

resource "terraform_data" "worker_replacement" {
  count = var.worker_instance_count
  input = sha256(local.cloud_init_worker)
}

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
    replace_triggered_by  = [terraform_data.api_replacement[count.index]]
  }
}

# Combined Instance (API + Redis on one box)
# Flexible IP provides reliable outbound at boot (SG blocks all unsolicited inbound).
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
    replace_triggered_by = [terraform_data.combined_replacement[0]]
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
    replace_triggered_by  = [terraform_data.worker_replacement[count.index]]
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
  registry_image = "${var.container_image}:${var.container_image_tag}"
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
