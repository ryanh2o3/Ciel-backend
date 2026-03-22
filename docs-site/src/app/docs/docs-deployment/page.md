---
title: Docs deployment
---

This documentation site is a **Next.js** app with **static export** (`out/`). It is deployed to **Scaleway Object Storage**, then served on **`https://docs.ciel-social.eu`** via **Scaleway Edge Services** (managed TLS, cache) when Terraform has Edge enabled (default in dev).

---

## Build locally

```bash
cd docs-site
npm ci
npm run build
```

Open `out/` with any static file server, e.g. `npx serve out`.

---

## CI/CD

GitHub Actions workflow **`.github/workflows/docs.yml`** builds on push to `main` (and on pull requests without upload) and uploads `out/` with **`s3cmd`**, using **`scw object config get type=s3cmd`** for Scaleway-compatible credentials (no AWS CLI / STS).

**Secrets** (repository):

| Secret | Purpose |
|--------|---------|
| `DOCS_BUCKET_NAME` | Bucket name from Terraform output |
| `DOCS_SCW_ACCESS_KEY` | IAM key with write access to the docs bucket |
| `DOCS_SCW_SECRET_KEY` | Secret for the same key |

Optional: reuse project-wide `SCW_ACCESS_KEY` / `SCW_SECRET_KEY` if policy allows (less ideal).

---

## Infrastructure (Terraform)

Module **`terraform/modules/docs_site`** defines:

- Object bucket for static files  
- **Bucket website** configuration (`index.html`, `404.html`) for origin semantics  
- **Edge Services pipeline** (default): S3 backend → WAF (disabled) → route → cache → **TLS** (managed Let’s Encrypt) → **DNS stage** for your docs FQDN → head stage  
- **DNS** `docs` **CNAME** to **`<pipeline-id>.svc.edge.scw.cloud`** (via `dns_cname_target` output), managed by the **`dns`** module when `enable_docs_dns` is true  

Disable Edge per environment with **`enable_docs_edge_services = false`** (dev) and **`enable_edge_services = false`** in the module; the CNAME then falls back to the bucket website hostname (usually HTTP-only on the custom domain).

---

## HTTPS

With Edge enabled, Scaleway issues a **managed certificate** for the FQDN passed as **`docs_fqdn`** (e.g. `docs.ciel-social.eu`). Allow a few minutes after the first apply for provisioning. See **`terraform output docs_https_note`** and **`docs_public_url`**.

If the certificate stays pending, confirm the **`docs`** CNAME matches **`terraform output docs_dns_cname_target`** and re-apply after DNS propagates.

{% callout title="Verify" %}
After deploy, open `https://docs.ciel-social.eu` in a private window and confirm `index.html`, client-side navigation, and `404.html` fallback (trailing slashes).
{% /callout %}
