variable "project_id" {
  description = "Scaleway project ID"
  type        = string
}

variable "region" {
  description = "Scaleway region (e.g. fr-par)"
  type        = string
}

variable "environment" {
  description = "Environment name (dev, prod, …)"
  type        = string
}

variable "app_name" {
  description = "Application name prefix for resources"
  type        = string
  default     = "ciel"
}

variable "bucket_name" {
  description = "Explicit bucket name; if null, a unique name is generated"
  type        = string
  default     = null
}

variable "tags" {
  description = "Tags as list of \"key:value\" strings"
  type        = list(string)
  default     = []
}

variable "enable_edge_services" {
  description = "Provision Scaleway Edge Services (HTTPS + cache) in front of the docs bucket"
  type        = bool
  default     = true
}

# docs_fqdn must appear after enable_edge_services (validation references it).
variable "docs_fqdn" {
  description = "Public FQDN for documentation (e.g. docs.example.com). Required when enable_edge_services is true."
  type        = string
  default     = ""

  validation {
    condition     = !var.enable_edge_services || var.docs_fqdn != ""
    error_message = "docs_fqdn must be set when enable_edge_services is true."
  }
}
