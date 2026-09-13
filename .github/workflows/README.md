# GitHub Actions workflows

## Docs site (`docs.yml`)

Builds the Next.js static export under `docs-site/` and uploads `out/` with **`s3cmd`** + Scaleway CLI (`scw object config get type=s3cmd`). Runs on pushes to `main` that touch `docs-site/**`, on related pull requests (build only), and via **workflow_dispatch**.

**Secrets:** `DOCS_BUCKET_NAME`, `DOCS_SCW_ACCESS_KEY`, `DOCS_SCW_SECRET_KEY`, plus `SCW_PROJECT_ID` and `SCW_ORGANIZATION_ID` (same as the main deploy workflow) for the CLI. **deploy.yml** may try to set `DOCS_*` via `gh` (often fails without a PAT); set them from Terraform outputs if needed. See [docs-site/README.md](../../docs-site/README.md).

---

## Publish Docker images to GHCR

Three workflows push amd64 images for Unraid / k3d. No extra secrets beyond `GITHUB_TOKEN` (`packages: write`).

| Workflow | Image | Triggers |
|----------|-------|----------|
| [`docker-publish.yml`](docker-publish.yml) | `ghcr.io/<owner>/ciel-backend` | Rust `src/`, migrations, `Dockerfile`, … |
| [`docker-publish-spring.yml`](docker-publish-spring.yml) | `ghcr.io/<owner>/ciel-api-spring` | `spring/**` |
| [`docker-publish-dotnet.yml`](docker-publish-dotnet.yml) | `ghcr.io/<owner>/ciel-api-dotnet` | `dotnet/**` |

Each runs tests, then Buildx push with tags `:main`, `:latest`, `:sha-<short>` (on `main` or **workflow_dispatch**).

### Unraid / k3d consumption

- **Compose (Rust-only legacy):** `CIEL_IMAGE=ghcr.io/ryanh2o3/ciel-backend:main` then `docker compose … pull/up`
- **Polyglot k8s:** manifests in [`k8s/`](../../k8s/) already reference GHCR `:main` — see [docs/UNRAID_K3S.md](../../docs/UNRAID_K3S.md)

After first publish, set each GHCR package **Public** (or configure `imagePullSecrets` + a `read:packages` PAT). Details: [docs/UNRAID_DEPLOY.md](../../docs/UNRAID_DEPLOY.md).

---

## Scaleway Terraform CI/CD (`deploy.yml.disabled`) — DISABLED

**Disabled.** Renamed from `deploy.yml` so GitHub Actions does not run Scaleway image builds or Terraform apply. Active publish path is [`docker-publish.yml`](docker-publish.yml) → GHCR for Unraid. To re-enable, rename back to `deploy.yml`.

---

### Original description (reference)

This workflow automated deployment of the Ciel backend to Scaleway infrastructure.

## Workflow Overview

The workflow consists of several jobs:

1. **build-and-push**: Builds Docker image and pushes to Scaleway Container Registry
2. **terraform-plan**: Runs Terraform plan for all environments (dev, staging, prod)
3. **terraform-apply**: Applies Terraform changes (only on main branch or manual trigger)
4. **database-migrations**: Runs database migrations against the deployed databases
5. **notify**: Sends Slack notification about deployment status

## Triggers

- **Push to main branch**: Automatically deploys to all environments
- **Pull Requests**: Runs Terraform plan only (no apply)
- **Manual workflow_dispatch**: Allows selecting specific environment and force apply

## Required GitHub Secrets

The workflow requires the following secrets to be set in your GitHub repository:

### Scaleway Credentials
- `SCW_ACCESS_KEY`: Scaleway access key
- `SCW_SECRET_KEY`: Scaleway secret key
- `SCW_PROJECT_ID`: Scaleway project ID

### Database Credentials
- `DB_ADMIN_PASSWORD`: Database admin password
- `DB_USER_PASSWORD`: Database user password

### Application Secrets
- `PASETO_ACCESS_KEY`: PASETO access key (base64, 32 bytes)
- `PASETO_REFRESH_KEY`: PASETO refresh key (base64, 32 bytes)
- `ADMIN_TOKEN`: Optional admin token for initial setup

### Notification (Optional)
- `SLACK_WEBHOOK_URL`: Slack webhook URL for deployment notifications

## Environment Variables

The workflow uses the following environment variables:

- `SCW_REGION`: fr-par
- `SCW_ZONE`: fr-par-1
- `CONTAINER_REGISTRY`: rg.fr-par.scw.cloud/ciel
- `DOCKER_IMAGE_NAME`: ciel-backend

## Usage

### For Development

1. Create a pull request with your changes
2. The workflow will run Terraform plan for all environments
3. Review the plan output in the GitHub Actions logs

### For Deployment

1. Push to main branch for automatic deployment
2. Or manually trigger the workflow from GitHub Actions UI
3. Select the environment(s) to deploy
4. Monitor the deployment progress

### Manual Override

To force a deployment even when there are no changes:

1. Go to GitHub Actions
2. Select "Scaleway Terraform CI/CD" workflow
3. Click "Run workflow"
4. Select environment and check "Force terraform apply"

## Security Notes

- All sensitive data is passed through GitHub Secrets
- Terraform state is stored in Scaleway S3-compatible storage
- Production environment requires manual approval
- Pull requests never apply changes, only plan

## Troubleshooting

- **Terraform init failures**: Ensure the S3 bucket for remote state exists
- **Authentication errors**: Verify all secrets are correctly set
- **Plan failures**: Check Terraform variable values and syntax
- **Apply failures**: Review the plan output and infrastructure constraints