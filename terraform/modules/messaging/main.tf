locals {
  queue_name = var.queue_name != null ? var.queue_name : "${var.app_name}-media-jobs-${var.environment}"
  dlq_name   = "${local.queue_name}-dlq"
}

# Enable SQS for the project
resource "scaleway_mnq_sqs" "main" {
  project_id = var.project_id
  region     = var.region
}

# SQS Credentials
resource "scaleway_mnq_sqs_credentials" "main" {
  project_id = var.project_id
  region     = var.region
  name       = "${var.app_name}-sqs-${var.environment}"

  permissions {
    can_manage  = true
    can_receive = true
    can_publish = true
  }

  depends_on = [scaleway_mnq_sqs.main]
}

# Dead Letter Queue (optional)
resource "scaleway_mnq_sqs_queue" "dlq" {
  count = var.enable_dlq ? 1 : 0

  project_id = var.project_id
  region     = var.region
  name       = local.dlq_name

  sqs_endpoint       = scaleway_mnq_sqs.main.endpoint
  access_key         = scaleway_mnq_sqs_credentials.main.access_key
  secret_key         = scaleway_mnq_sqs_credentials.main.secret_key

  message_max_age            = var.dlq_retention_seconds
  visibility_timeout_seconds = 30

  depends_on = [scaleway_mnq_sqs_credentials.main]
}

# Main Media Jobs Queue
resource "scaleway_mnq_sqs_queue" "main" {
  project_id = var.project_id
  region     = var.region
  name       = local.queue_name

  sqs_endpoint = scaleway_mnq_sqs.main.endpoint
  access_key   = scaleway_mnq_sqs_credentials.main.access_key
  secret_key   = scaleway_mnq_sqs_credentials.main.secret_key

  message_max_age             = var.message_retention_seconds
  visibility_timeout_seconds  = var.visibility_timeout
  receive_wait_time_seconds   = var.receive_wait_time

  dynamic "dead_letter_queue" {
    for_each = var.enable_dlq ? [1] : []
    content {
      id                = scaleway_mnq_sqs_queue.dlq[0].id
      max_receive_count = var.dlq_max_receive_count
    }
  }

  depends_on = [scaleway_mnq_sqs_credentials.main]
}
