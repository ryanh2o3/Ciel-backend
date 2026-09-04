# Unraid deploy (Cloudflare Tunnel + Scaleway DNS)

Run the Ciel API + media worker on an Unraid box without opening router ports. Public HTTPS reaches the stack through **Cloudflare Tunnel**; DNS for `ciel-social.eu` stays on **Scaleway Domains**.

Homelab hostnames (avoid clobbering live Scaleway `api.` / `media.` records):

| Role | Hostname |
|------|----------|
| API | `home-api.ciel-social.eu` |
| Media (MinIO / S3) | `home-media.ciel-social.eu` |

```text
Phone → https://home-api… / https://home-media…
      → Cloudflare edge
      → cloudflared (outbound from Unraid)
      → api:8080 / minio:9000
```

## Prerequisites

- Unraid with Docker (Compose V2 plugin or CLI)
- Domain on Scaleway Domains & DNS (e.g. `ciel-social.eu`)
- Free [Cloudflare](https://dash.cloudflare.com/) account (Zero Trust / Tunnels)
- Repo checkout on the Unraid host for compose files, migrations, and `docker/` scripts (api/worker images are **pulled** from GHCR — no Rust build on Unraid)

## 1. Unraid shares

Create shares (or folders):

- `/mnt/user/appdata/ciel` — Postgres + Redis data
- `/mnt/user/appdata/ciel-media` — MinIO object storage

Clone this backend onto the array, e.g. `/mnt/user/appdata/ciel/Ciel-backend`.

## 2. Cloudflare Tunnel (dashboard only — do not install a separate Unraid app)

You do **not** install Cloudflare Tunnel from the Unraid Apps/CA store for this setup. The connector is the `cloudflared` service inside [`docker-compose.unraid.yml`](../docker-compose.unraid.yml). It starts automatically in **step 6** when you `docker compose … up -d`, after you put the token in `.env` (**step 4**).

**This step is only the Cloudflare website side:**

1. Cloudflare Dashboard → **Zero Trust** → **Networks** → **Tunnels** → **Create a tunnel** (Cloudflared).
2. Name it (e.g. `ciel-unraid`) and save.
3. Cloudflare will show **install / run connector** commands (Docker, Linux, etc.) — there is usually **no separate “copy token” button**. That is normal.
4. **Extract the token from any install command:**
   - Pick the **Docker** tab (or any OS — the token is the same).
   - Copy the whole command into a text editor (do **not** run it on Unraid).
   - Find `--token` (or `cloudflared service install `). The value after it is a long string starting with `eyJ…`.
   - That string alone is `TUNNEL_TOKEN` for step 4.

   Example (token abbreviated):

   ```text
   docker run cloudflare/cloudflared:latest tunnel --no-autoupdate run --token eyJhIjoiNWFiNGU5Z...
   ```

   → put only `eyJhIjoiNWFiNGU5Z...` in `.env` as `TUNNEL_TOKEN=...`

5. Skip actually installing that connector on Unraid — Compose already runs `cloudflare/cloudflared:latest` with the token.
6. If you already closed the wizard: open the tunnel → **Overview** / **Edit** → **Add a connector** (or similar) to reveal the install command again, then extract `--token` the same way.
7. Under **Public Hostname**, add:

| Public hostname | Service | Notes |
|-----------------|---------|--------|
| `home-api.ciel-social.eu` | `http://api:8080` | Same Docker network as compose |
| `home-media.ciel-social.eu` | `http://minio:9000` | Path-style S3 API at domain root |

8. Note the tunnel UUID (shown in the tunnel details / CNAME target). Cloudflare shows a target like:

   `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx.cfargotunnel.com`

`cloudflared` in Compose joins the project network, so use service names `api` and `minio` (not Unraid host IPs).

If you instead run `cloudflared` as a separate Unraid container on `bridge`, point services at host IPs/ports and publish those ports — prefer the in-compose service.

## 3. Scaleway DNS CNAMEs

In **Scaleway Console → Domains & DNS → `ciel-social.eu`**, add:

| Name | Type | Value |
|------|------|--------|
| `home-api` | CNAME | `<TUNNEL_UUID>.cfargotunnel.com.` |
| `home-media` | CNAME | `<TUNNEL_UUID>.cfargotunnel.com.` |

TTL can stay default. Do **not** move Scaleway nameservers to Cloudflare unless you want Cloudflare to own all DNS.

Verify:

```bash
dig +short home-api.ciel-social.eu CNAME
dig +short home-media.ciel-social.eu CNAME
```

## 4. Secrets and `.env`

On the Unraid host, in the backend directory:

```bash
cp .env.unraid.example .env
openssl rand -base64 32   # → PASETO_ACCESS_KEY
openssl rand -base64 32   # → PASETO_REFRESH_KEY
```

Edit `.env`:

- Set `TUNNEL_TOKEN` from step 2 (the `eyJ…` string after `--token` in Cloudflare’s install command — not the whole `docker run` line)
- Set strong `POSTGRES_PASSWORD` and `MINIO_ROOT_PASSWORD`
- Confirm paths `CIEL_DATA_DIR` / `CIEL_MEDIA_DIR`
- Set `CIEL_IMAGE` (default `ghcr.io/ryanh2o3/ciel-backend:main`)
- Keep:

```bash
S3_ENDPOINT=https://home-media.ciel-social.eu
S3_PUBLIC_ENDPOINT=
```

Upload and download URLs are **signed against `S3_ENDPOINT`**. Using the public HTTPS media hostname matches Scaleway production (public object endpoint) so phones can PUT/GET without relying on URL host rewriting. The API and worker reach MinIO by hairpinning through the tunnel, so DNS + tunnel must be live before media flows work.

`TRUSTED_PROXY_CIDRS` must stay set so Cloudflare’s `X-Forwarded-Proto: https` is trusted (otherwise non-localhost HTTP is rejected).

## 5. GHCR image (CI)

On every push to `main` that touches backend sources, [`.github/workflows/docker-publish.yml`](../.github/workflows/docker-publish.yml) runs tests, then builds and pushes:

| Tag | Meaning |
|-----|---------|
| `ghcr.io/ryanh2o3/ciel-backend:main` | Latest successful main build |
| `ghcr.io/ryanh2o3/ciel-backend:latest` | Same as `:main` |
| `ghcr.io/ryanh2o3/ciel-backend:sha-<short>` | Immutable pin for rollbacks |

### Package visibility

After the first successful publish, open **GitHub → Packages → ciel-backend**:

- **Public package** — Unraid can `docker pull` with no login (simplest for a personal homelab).
- **Private package** — create a classic PAT with `read:packages`, then on Unraid:

```bash
echo "$GHCR_TOKEN" | docker login ghcr.io -u ryanh2o3 --password-stdin
```

Store the login in Unraid’s Docker credentials so pulls survive reboots.

## 6. Start the stack (this is when Cloudflare Tunnel is installed on Unraid)

DNS and tunnel public hostnames should already exist. Prefer pulling a pre-built image (no compile on Unraid).

`docker compose … up -d` pulls and starts **`cloudflared`** with `TUNNEL_TOKEN` from `.env` — that is the Unraid-side Tunnel install. Confirm it is healthy with `docker compose … logs -f cloudflared` (should show a registered connection, not “invalid token”).

```bash
cd /path/to/Ciel-backend
docker compose -f docker-compose.unraid.yml --env-file .env pull
docker compose -f docker-compose.unraid.yml --env-file .env up -d
```

### Manual updates (after CI finishes on main)

```bash
git pull   # compose / migrations / docker scripts
docker compose -f docker-compose.unraid.yml --env-file .env pull api worker
docker compose -f docker-compose.unraid.yml --env-file .env up -d api worker
# If migrations changed:
docker compose -f docker-compose.unraid.yml --env-file .env run --rm migrate
```

To roll back, set `CIEL_IMAGE=ghcr.io/ryanh2o3/ciel-backend:sha-<old>` in `.env`, then `pull` + `up -d` again.

Useful commands:

```bash
docker compose -f docker-compose.unraid.yml --env-file .env ps
docker compose -f docker-compose.unraid.yml --env-file .env logs -f api worker cloudflared
docker compose -f docker-compose.unraid.yml --env-file .env down
```

## 7. Client apps

Point both apps at the Unraid API (include `/v1`):

- **iOS** — `PicShare-ios/PicShare-ios/App/AppContainer.swift`  
  `baseURL: URL(string: "https://home-api.ciel-social.eu/v1")!`
- **Android** — `PicShare-android/.../NetworkModule.kt`  
  `BASE_URL = "https://home-api.ciel-social.eu/v1/"`

Devices must resolve both `home-api` and `home-media` (tunnel covers both).

## 8. Smoke checklist

1. `curl -sS https://home-api.ciel-social.eu/health` → healthy JSON / 200  
2. Register or login via the app (or API) against `https://home-api.ciel-social.eu/v1`  
3. Create a media upload; the returned `upload_url` host should be `home-media.ciel-social.eu`  
4. Complete upload; worker logs show job processing; feed/post shows thumbnails from `home-media…`  
5. Confirm Postgres/Redis/MinIO are **not** published on the WAN (compose does not map them publicly)

## Architecture notes

| Service | Role |
|---------|------|
| `db` | Postgres 16 |
| `redis` | Cache |
| `minio` + `minio-init` | S3-compatible media; bucket + CORS |
| `elasticmq` | SQS-compatible `ciel-media-jobs` queue |
| `migrate` | Applies `migrations/*.sql` once |
| `api` / `worker` | Same GHCR image; `APP_MODE=api` / `worker` |
| `cloudflared` | Outbound tunnel only |

Local development still uses [`docker-compose.yml`](../docker-compose.yml) (LocalStack + local `build:`). Do not mix the two compose files on the same data dirs unless you intend to.

## Troubleshooting

| Symptom | Likely cause |
|---------|----------------|
| `pull access denied` from ghcr.io | Package private; `docker login ghcr.io` with a `read:packages` PAT, or make the package public |
| Image not found / old digest | CI still running or failed; check Actions → **Publish Docker image (GHCR)** |
| API 403 on requests | `TRUSTED_PROXY_CIDRS` empty; set Docker/LAN CIDRs |
| `SignatureDoesNotMatch` on media | `S3_ENDPOINT` must be exactly `https://home-media…` (scheme + host); MinIO `MINIO_SERVER_URL` follows that env |
| Upload URL is `http://minio:9000` | `.env` still using internal endpoint; use public `S3_ENDPOINT` |
| API up but media fails | Tunnel/DNS for `home-media` not ready; check `cloudflared` logs |
| Queue errors | `elasticmq` unhealthy; check `docker/elasticmq/elasticmq.conf` mount |

## Optional: use production names

If Scaleway cloud API DNS is unused, change tunnel hostnames and Scaleway CNAMEs to `api.ciel-social.eu` / `media.ciel-social.eu`, update `S3_ENDPOINT`, and point the mobile apps at `https://api.ciel-social.eu/v1`.
