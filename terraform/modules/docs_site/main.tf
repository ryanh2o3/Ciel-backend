resource "random_id" "docs_bucket_suffix" {
  count       = var.bucket_name == null ? 1 : 0
  byte_length = 3
}

locals {
  bucket_name = var.bucket_name != null ? var.bucket_name : "${var.app_name}-docs-${var.environment}-${random_id.docs_bucket_suffix[0].hex}"
  tag_map = {
    for tag in var.tags :
    split(":", tag)[0] => split(":", tag)[1]
    if length(split(":", tag)) == 2
  }
  edge_count = var.enable_edge_services && var.docs_fqdn != "" ? 1 : 0
}

resource "scaleway_object_bucket" "docs" {
  name   = local.bucket_name
  region = var.region

  tags = merge(
    local.tag_map,
    { environment = var.environment, purpose = "static-docs" },
  )
}

resource "scaleway_object_bucket_website_configuration" "docs" {
  bucket = scaleway_object_bucket.docs.name
  region = var.region

  index_document {
    suffix = "index.html"
  }

  error_document {
    key = "404.html"
  }
}

resource "scaleway_iam_application" "docs_deploy" {
  name        = "${var.app_name}-docs-deploy-${var.environment}"
  description = "CI deploy key for static docs bucket ${local.bucket_name}"
}

resource "scaleway_iam_policy" "docs_deploy" {
  name           = "${var.app_name}-docs-deploy-policy-${var.environment}"
  description    = "Object Storage read/write/delete for docs bucket ${local.bucket_name}"
  application_id = scaleway_iam_application.docs_deploy.id

  rule {
    project_ids = [var.project_id]
    permission_set_names = [
      "ObjectStorageObjectsRead",
      "ObjectStorageObjectsWrite",
      "ObjectStorageObjectsDelete",
    ]
  }
}

resource "scaleway_iam_api_key" "docs_deploy" {
  application_id = scaleway_iam_application.docs_deploy.id
  description    = "Terraform-managed key for uploading PicShare docs to ${local.bucket_name}"
}

# Public read policy for static website.
# Keep bucket policy in 2012-10-17 mode to support project_id principals.
# CI deploy app permissions are granted via IAM policy above.
resource "scaleway_object_bucket_policy" "docs" {
  bucket = scaleway_object_bucket.docs.name
  region = var.region

  policy = jsonencode({
    Version = "2012-10-17"
    Id      = "DocsSiteBucketPolicy"
    Statement = [
      {
        Sid    = "AllowProjectProvisioningAccess"
        Effect = "Allow"
        Principal = {
          SCW = "project_id:${var.project_id}"
        }
        Action = ["s3:*"]
        Resource = [
          scaleway_object_bucket.docs.name,
          "${scaleway_object_bucket.docs.name}/*",
        ]
      },
      {
        Sid       = "AllowPublicReadDocs"
        Effect    = "Allow"
        Principal = "*"
        Action    = ["s3:GetObject"]
        Resource  = ["${scaleway_object_bucket.docs.name}/*"]
      },
    ]
  })
}

# -----------------------------------------------------------------------------
# Edge Services — HTTPS + cache in front of the static bucket
# Chain matches provider docs: backend → waf → route → cache → tls → dns → head
# -----------------------------------------------------------------------------

resource "scaleway_edge_services_pipeline" "docs" {
  count = local.edge_count

  name        = "${var.app_name}-docs-${var.environment}"
  description = "PicShare documentation (S3 origin)"
}

resource "scaleway_edge_services_backend_stage" "docs" {
  count = local.edge_count

  pipeline_id = scaleway_edge_services_pipeline.docs[count.index].id

  s3_backend_config {
    bucket_name   = scaleway_object_bucket.docs.name
    bucket_region = var.region
    # Use website endpoint semantics (index.html, error doc). Without this, Edge
    # hits the S3 API and GET / is treated as ListBucket → public AccessDenied.
    is_website = true
  }
}

resource "scaleway_edge_services_waf_stage" "docs" {
  count = local.edge_count

  pipeline_id      = scaleway_edge_services_pipeline.docs[count.index].id
  backend_stage_id = scaleway_edge_services_backend_stage.docs[count.index].id
  mode             = "disable"
  paranoia_level   = 1
}

resource "scaleway_edge_services_route_stage" "docs" {
  count = local.edge_count

  pipeline_id  = scaleway_edge_services_pipeline.docs[count.index].id
  waf_stage_id = scaleway_edge_services_waf_stage.docs[count.index].id

  rule {
    backend_stage_id = scaleway_edge_services_backend_stage.docs[count.index].id
    rule_http_match {
      path_filter {
        path_filter_type = "regex"
        value            = ".*"
      }
    }
  }
}

resource "scaleway_edge_services_cache_stage" "docs" {
  count = local.edge_count

  pipeline_id    = scaleway_edge_services_pipeline.docs[count.index].id
  route_stage_id = scaleway_edge_services_route_stage.docs[count.index].id
}

resource "scaleway_edge_services_tls_stage" "docs" {
  count = local.edge_count

  pipeline_id         = scaleway_edge_services_pipeline.docs[count.index].id
  cache_stage_id      = scaleway_edge_services_cache_stage.docs[count.index].id
  managed_certificate = true
}

resource "scaleway_edge_services_dns_stage" "docs" {
  count = local.edge_count

  pipeline_id  = scaleway_edge_services_pipeline.docs[count.index].id
  tls_stage_id = scaleway_edge_services_tls_stage.docs[count.index].id
  fqdns        = [var.docs_fqdn]
}

resource "scaleway_edge_services_head_stage" "docs" {
  count = local.edge_count

  pipeline_id   = scaleway_edge_services_pipeline.docs[count.index].id
  head_stage_id = scaleway_edge_services_dns_stage.docs[count.index].id
}
