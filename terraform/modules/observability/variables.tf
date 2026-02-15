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

variable "retention_days" {
  description = "Data retention period for Cockpit data sources (1-365 days)"
  type        = number
  default     = 31
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
