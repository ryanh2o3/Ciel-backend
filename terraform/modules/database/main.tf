# Managed PostgreSQL Instance
resource "scaleway_rdb_instance" "main" {
  name   = "${var.app_name}-db-${var.environment}"
  region = var.region

  node_type          = var.db_node_type
  engine             = var.db_engine
  is_ha_cluster      = var.enable_ha
  disable_backup     = false
  volume_type        = "bssd"
  volume_size_in_gb  = var.volume_size_in_gb

  backup_schedule_frequency = var.backup_schedule_frequency
  backup_schedule_retention = var.backup_schedule_retention

  # Private network endpoint
  private_network {
    pn_id       = var.private_network_id
    enable_ipam = true
  }

  # PostgreSQL settings
  init_settings = {
    work_mem             = var.db_settings["work_mem"]
    max_connections      = var.db_settings["max_connections"]
    effective_cache_size = var.db_settings["effective_cache_size"]
  }

  tags = concat(var.tags, ["environment:${var.environment}"])

  lifecycle {
    prevent_destroy = true
  }
}

# Database
resource "scaleway_rdb_database" "main" {
  instance_id = scaleway_rdb_instance.main.id
  name        = var.db_name

  lifecycle {
    prevent_destroy = true
  }
}

# Application User
resource "scaleway_rdb_user" "app" {
  instance_id = scaleway_rdb_instance.main.id
  name        = var.db_user
  password    = var.db_user_password
  is_admin    = false
}

# Privileges for application user
resource "scaleway_rdb_privilege" "app" {
  instance_id   = scaleway_rdb_instance.main.id
  database_name = scaleway_rdb_database.main.name
  user_name     = scaleway_rdb_user.app.name
  permission    = var.db_user_permission

  depends_on = [
    scaleway_rdb_database.main,
    scaleway_rdb_user.app
  ]
}

# Read Replicas
resource "scaleway_rdb_read_replica" "main" {
  count = var.read_replica_count

  instance_id = scaleway_rdb_instance.main.id
  region      = var.region

  private_network {
    private_network_id = var.private_network_id
    enable_ipam        = true
  }
}

# Get the private endpoint
data "scaleway_rdb_instance" "main" {
  instance_id = scaleway_rdb_instance.main.id
  region      = var.region

  depends_on = [scaleway_rdb_instance.main]
}
