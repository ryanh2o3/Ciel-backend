# Remote state configuration for dev environment
# Create the bucket manually before running terraform init

terraform {
  backend "s3" {
    bucket   = "ciel-terraform-state"
    key      = "dev/terraform.tfstate"
    region   = "fr-par"
    endpoints = {
      s3 = "https://s3.fr-par.scw.cloud"
    }
    encrypt  = false
    use_lockfile = true

    # Skip validations since we're using Scaleway S3-compatible endpoint
    skip_credentials_validation  = true
    skip_region_validation       = true
    skip_metadata_api_check      = true
    skip_requesting_account_id   = true
    skip_s3_checksum             = true
  }
}
