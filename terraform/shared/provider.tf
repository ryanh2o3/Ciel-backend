provider "scaleway" {
  zone            = var.zone
  region          = var.region
  project_id      = var.project_id
  organization_id = var.organization_id
}

variable "organization_id" {
  description = "Scaleway organization ID"
  type        = string
}

variable "zone" {
  description = "Scaleway zone"
  type        = string
  default     = "fr-par-1"
}

variable "region" {
  description = "Scaleway region"
  type        = string
  default     = "fr-par"
}

variable "project_id" {
  description = "Scaleway project ID"
  type        = string
}
