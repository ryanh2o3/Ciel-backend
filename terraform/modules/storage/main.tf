# Generate unique bucket name if not provided
resource "random_id" "bucket_suffix" {
  byte_length = 4
}

locals {
  bucket_name = var.bucket_name != null ? var.bucket_name : "${var.app_name}-media-${var.environment}-${random_id.bucket_suffix.hex}"
}

# Object Storage Bucket
resource "scaleway_object_bucket" "media" {
  name   = local.bucket_name
  region = var.region

  # CORS configuration (optional)
  dynamic "cors_rule" {
    for_each = length(var.cors_allowed_origins) > 0 ? [1] : []
    content {
      allowed_headers = ["*"]
      allowed_methods = var.cors_allowed_methods
      allowed_origins = var.cors_allowed_origins
      max_age_seconds = var.cors_max_age_seconds
    }
  }

  # Versioning (optional)
  versioning {
    enabled = var.enable_versioning
  }

  # Lifecycle rules
  lifecycle_rule {
    id      = "abort-incomplete-multipart"
    enabled = true

    abort_incomplete_multipart_upload_days = var.incomplete_multipart_days
  }

  # Glacier transition for originals (optional)
  dynamic "lifecycle_rule" {
    for_each = var.enable_glacier_transition ? [1] : []
    content {
      id      = "glacier-transition"
      enabled = true
      prefix  = var.glacier_prefix

      transition {
        days          = var.glacier_transition_days
        storage_class = "GLACIER"
      }
    }
  }

  tags = merge(
    { for tag in var.tags : split(":", tag)[0] => split(":", tag)[1] if length(split(":", tag)) == 2 },
    { environment = var.environment }
  )

  lifecycle {
    prevent_destroy = true
  }
}

# IAM Application for S3 access
resource "scaleway_iam_application" "s3_access" {
  name        = "${var.app_name}-s3-${var.environment}"
  description = "S3 access for ${var.app_name} ${var.environment}"
}

# IAM Policy for S3 access
resource "scaleway_iam_policy" "s3_access" {
  name           = "${var.app_name}-s3-policy-${var.environment}"
  description    = "S3 access policy for ${var.app_name} ${var.environment}"
  application_id = scaleway_iam_application.s3_access.id

  rule {
    project_ids = [var.project_id]
    permission_set_names = [
      "ObjectStorageObjectsRead",
      "ObjectStorageObjectsWrite",
      "ObjectStorageObjectsDelete",
    ]
  }
}

# API Key for S3 access
resource "scaleway_iam_api_key" "s3_access" {
  application_id = scaleway_iam_application.s3_access.id
  description    = "S3 access key for ${var.app_name} ${var.environment}"
}

# Bucket policy for public read access to processed images (optional)
resource "scaleway_object_bucket_policy" "media" {
  bucket = scaleway_object_bucket.media.name
  region = var.region

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid       = "AllowProjectManagement"
        Effect    = "Allow"
        Principal = { SCW = "project_id:${var.project_id}" }
        Action    = ["s3:*"]
        Resource  = [
          scaleway_object_bucket.media.name,
          "${scaleway_object_bucket.media.name}/*"
        ]
      },
      {
        Sid       = "AllowPublicRead"
        Effect    = "Allow"
        Principal = "*"
        Action    = ["s3:GetObject"]
        Resource  = ["${scaleway_object_bucket.media.name}/processed/*"]
      }
    ]
  })
}
