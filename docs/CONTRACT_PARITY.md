# Cross-impl parity checklist

Run the same sequence against each backend via `X-Ciel-Backend: rust|spring|dotnet` (or direct service URL). Assert status codes and JSON field shapes match (normalize timestamps / UUIDs / signed URLs as noted).

Automated runner: [`../scripts/parity-check.sh`](../scripts/parity-check.sh).

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
