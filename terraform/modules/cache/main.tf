locals {
  cloud_init_redis = var.enabled && !var.use_managed_redis ? templatefile("${path.module}/cloud-init-redis.yaml", {
    redis_password   = var.redis_password
    maxmemory_mb     = var.redis_maxmemory_mb
    maxmemory_policy = var.redis_maxmemory_policy
  }) : ""
}

# Self-managed Redis instance
resource "scaleway_instance_server" "redis" {
  count = var.enabled && !var.use_managed_redis ? 1 : 0

  name  = "${var.app_name}-redis-${var.environment}"
  type  = var.redis_instance_type
  image = "debian_bookworm"
  zone  = var.zone

  security_group_id = var.security_group_id

  private_network {
    pn_id = var.private_network_id
  }

  user_data = {
    cloud-init = local.cloud_init_redis
  }

  tags = concat(var.tags, [
    "environment:${var.environment}",
    "role:redis",
    "app:${var.app_name}"
  ])
}

# Managed Redis cluster
resource "scaleway_redis_cluster" "main" {
  count = var.enabled && var.use_managed_redis ? 1 : 0

  name         = "${var.app_name}-redis-${var.environment}"
  version      = var.managed_redis_version
  node_type    = var.managed_redis_node_type
  user_name    = "${var.app_name}_${var.environment}"
  cluster_size = var.managed_redis_cluster_size
  zone         = var.zone
  tls_enabled  = var.managed_redis_tls_enabled
  password     = var.redis_password

  private_network {
    id = var.private_network_id
  }

  settings = {
    maxmemory-policy = var.redis_maxmemory_policy
  }

  tags = concat(var.tags, ["environment:${var.environment}"])
}
