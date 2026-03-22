# PicShare documentation (Syntax template)

Static documentation for **PicShare** and the **Ciel** backend. Built with [Tailwind Plus Syntax](https://tailwindcss.com/plus/templates/syntax), [Next.js](https://nextjs.org), and [Markdoc](https://markdoc.io).

## Local development

```bash
npm install
npm run dev
```

Open [http://localhost:3000](http://localhost:3000).

## Production build (static export)

The site is configured with `output: 'export'` and `trailingSlash: true`. The deployable artifact is the **`out/`** directory.

```bash
npm ci
npm run build
npx serve out
```

## Deploy (Scaleway)

1. **Terraform** (`terraform/environments/dev`) provisions a dedicated Object Storage bucket, bucket website config (`index.html` / `404.html`), IAM keys for CI, and optional `docs` **CNAME** on `ciel-social.eu`. See Terraform outputs: `docs_bucket_name`, `docs_deploy_*`, `docs_https_note`.
2. **GitHub Actions** (`.github/workflows/docs.yml`) runs on changes under `docs-site/` and syncs `out/` with `aws s3 sync` to Scaleway (`https://s3.fr-par.scw.cloud`).

Repository secrets (for `.github/workflows/docs.yml`):

| Secret | Description |
|--------|-------------|
| `DOCS_BUCKET_NAME` | Object Storage bucket name |
| `DOCS_SCW_ACCESS_KEY` | IAM access key that can write the bucket |
| `DOCS_SCW_SECRET_KEY` | IAM secret key |

**Without a local Terraform run:** after a successful **Scaleway Terraform CI/CD** apply on `main`, the **Sync docs-site secrets from Terraform outputs** step in `.github/workflows/deploy.yml` tries to set these three secrets via the GitHub API (values piped from `terraform output -raw`, not printed in logs). That requires the workflow job permission `secrets: write` (default for the repo token on same-repo pushes; some orgs restrict this — if the step fails, set the secrets manually or fix org policy).

**HTTPS:** Terraform provisions **Scaleway Edge Services** (managed Let’s Encrypt) for `docs_fqdn` by default. See `terraform/modules/docs_site` and `terraform output docs_public_url` / `docs_https_note`. Turn off with `enable_docs_edge_services` (dev) or module `enable_edge_services` if you need bucket-only HTTP.

## Site configuration

Optional build-time URLs (defaults in `src/lib/site.ts`):

- `NEXT_PUBLIC_GITHUB_REPO_URL` — header / hero “Source on GitHub” link
- `NEXT_PUBLIC_PRODUCT_SITE_URL` — reserved for product links (default `https://ciel-social.eu`)

## Search

FlexSearch indexes `**/page.md` under `src/app`. Paths use trailing slashes to match static hosting. Adjust `src/markdoc/search.mjs` if needed.

## License

The Syntax template is a commercial [Tailwind Plus](https://tailwindcss.com/plus/license) product.
