variable "project_id" {
  description = "Scaleway project ID"
  type        = string
}

variable "region" {
  description = "Scaleway region"
  type        = string
  default     = "fr-par"
}

variable "zone" {
  description = "Scaleway zone"
  type        = string
  default     = "fr-par-1"
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
}

variable "app_name" {
  description = "Application name"
  type        = string
  default     = "ciel"
}

variable "enable_cockpit" {
  description = "Enable Scaleway Cockpit for monitoring"
  type        = bool
  default     = true
}

variable "grafana_user_role" {
  description = "Grafana user role (viewer, editor)"
  type        = string
  default     = "viewer"
}

variable "enable_alerts" {
  description = "Enable alert manager"
  type        = bool
  default     = true
}

variable "alert_contact_emails" {
  description = "Email addresses for alerts"
  type        = list(string)
  default     = []
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = list(string)
  default     = []
}
