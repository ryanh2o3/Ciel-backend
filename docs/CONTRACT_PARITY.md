# Cross-impl parity checklist

Run the same sequence against each backend via `X-Ciel-Backend: rust|spring|dotnet` (or direct service URL). Assert status codes and JSON field shapes match (normalize timestamps / UUIDs / signed URLs as noted).

## Automated local suite

Run `bash scripts/run-parity-local.sh` from the repository root. Requires Rust, Java 21, Maven, .NET 8, Python 3, PostgreSQL server binaries and Redis. Set `PG_BIN` and `JAVA_BIN` if binaries are not on PATH. The wrapper selects Homebrew Java 21 when available.

The suite builds all three native APIs, runs Spring/.NET unit tests, verifies all 67 method/path pairs, and starts disposable local PostgreSQL, Redis, and an S3/SQS protocol test double. It creates its own accounts, invites and keys; no production token or configured deployment is used. Processes stop on exit; the printed temporary directory retains logs.

Each backend independently exercises auth, profiles, posts, social relationships, search, notifications, invites, devices, trust, moderation, stories, highlights and media lifecycle. Status codes and normalized response bodies must match. Nine issuer/verifier combinations test access tokens, refresh rotation and replay revocation across backends against the same database. UUIDs, timestamps, secrets and signed URLs are normalized; business fields and error messages are compared. `X-Ciel-Served-By` intentionally differs. Metrics contain backend-local measurements.

CI runs this suite in `.github/workflows/api-parity.yml`. SQL in each port executes directly against the shared schema with its native database driver; there is no runtime Rust dependency for HTTP handlers.

The AWS test double verifies API/queue contracts, not real S3 signature acceptance or image processing. The deployment checklist below still requires real infrastructure. Existing deployed-stack smoke runner: [`../scripts/parity-check.sh`](../scripts/parity-check.sh).

## Deployment checklist

## Auth

- [ ] `GET /health` → `{"status":"ok"}` (or `"degraded"`); never plain text
- [ ] Timestamps are RFC3339 strings (e.g. `access_expires_at`), not epoch numbers
- [ ] `next_cursor` key present even when null on list endpoints
- [ ] `POST /v1/auth/login` — valid credentials → 200 + `access_token`, `refresh_token`, `access_expires_at`, `refresh_expires_at`
- [ ] `POST /v1/auth/login` — bad password → 401 `invalid credentials`
- [ ] `GET /v1/auth/me` with access → 200 user (includes `email`)
- [ ] `POST /v1/auth/refresh` → new token pair; old refresh cannot be reused successfully
- [ ] `POST /v1/auth/revoke` → 204; subsequent refresh fails
- [ ] Token minted on Rust validates on Spring/.NET and vice versa (shared keys)

## Users / posts / feed

- [ ] `POST /v1/users` with invite → 200/201 user
- [ ] `GET /v1/users/{id}` → public profile counts
- [ ] `POST /v1/posts` with owned `media_ids` → post with `media_ids`, `visibility`
- [ ] `GET /v1/feed` → `{ items, next_cursor }`; cursor round-trip
- [ ] `POST /v1/feed/refresh` → 204

## Media

- [ ] `POST /v1/media/upload` → `upload_id`, `object_key` under `uploads/...`, `upload_url`, `headers`
- [ ] Client PUT to presigned URL succeeds
- [ ] `POST /v1/media/upload/{id}/complete` → 202
- [ ] Rust worker produces `media/.../thumb.*` and `medium.*`
- [ ] `GET /v1/media/upload/{id}/status` → `ready` + `processed_media_id`
- [ ] `GET /v1/media/{id}` → media with URLs

## Opaque ingress

- [ ] No `X-Ciel-Backend` → responses from weighted pool; each has `X-Ciel-Served-By`
- [ ] `X-Ciel-Backend: rust` always hits Rust (etc.)
- [ ] Rate-limit keys shared: hitting limit on one impl blocks others for same user/action
