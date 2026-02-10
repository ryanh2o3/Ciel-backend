output "redis_host" {
  description = "Redis host address"
  value = !var.enabled ? null : (
    var.use_managed_redis ? (
      length(scaleway_redis_cluster.main) > 0 ? one([for pn in scaleway_redis_cluster.main[0].private_network : pn.endpoint_id]) : null
    ) : (
      length(scaleway_instance_server.redis) > 0 ? try(scaleway_instance_server.redis[0].private_ips[0].address, null) : null
    )
  )
}

output "redis_port" {
  description = "Redis port"
  value = var.enabled ? 6379 : null
}

output "redis_url" {
  description = "Full Redis connection URL"
  value = !var.enabled ? null : (
    var.use_managed_redis ? (
      length(scaleway_redis_cluster.main) > 0 ? (
        var.managed_redis_tls_enabled ?
          "rediss://:${var.redis_password}@${one([for pn in scaleway_redis_cluster.main[0].private_network : pn.endpoint_id])}:6379" :
          "redis://:${var.redis_password}@${one([for pn in scaleway_redis_cluster.main[0].private_network : pn.endpoint_id])}:6379"
      ) : null
    ) : (
      length(scaleway_instance_server.redis) > 0 ?
        "redis://:${var.redis_password}@${try(scaleway_instance_server.redis[0].private_ips[0].address, "")}:6379" : null
    )
  )
  sensitive = true
}

output "self_managed_instance_id" {
  description = "Self-managed Redis instance ID"
  value = !var.enabled ? null : (
    var.use_managed_redis ? null : (length(scaleway_instance_server.redis) > 0 ? scaleway_instance_server.redis[0].id : null)
  )
}

output "managed_cluster_id" {
  description = "Managed Redis cluster ID"
  value = !var.enabled ? null : (
    var.use_managed_redis ? (length(scaleway_redis_cluster.main) > 0 ? scaleway_redis_cluster.main[0].id : null) : null
  )
}

output "is_managed" {
  description = "Whether using managed Redis"
  value = var.enabled ? var.use_managed_redis : false
}

output "redis_use_tls" {
  description = "Whether Redis connections should use TLS"
  value = var.enabled ? (var.use_managed_redis ? var.managed_redis_tls_enabled : false) : false
}
