# Ciel cross-implementation contract

Shared contracts for Rust, Spring Boot, and ASP.NET Core API implementations behind one opaque ingress. **Postgres schema is owned by Rust migrations** — other stacks never invent DDL.

## HTTP

| Item | Rule |
|------|------|
| Base path | `/v1` |
| Errors | `{"error":"<message>"}` |
| Auth header | `Authorization: Bearer <access_token>` |
| Admin header | `x-admin-token: <ADMIN_TOKEN>` |
| Pagination | Query `limit` (default 30, max 200) + `cursor`; response `{ "items": [...], "next_cursor": "..." \| null }` — **`next_cursor` key always present** |
| Timestamps | RFC3339 strings (e.g. `2024-01-15T12:00:00Z`), never epoch numbers |
| Nulls | Match Rust serde: include `null` for `Option` fields unless Rust uses `skip_serializing_if` |
| Health | `GET /health` → `{"status":"ok"\|"degraded"}` (`application/json`) |
| Served-by | Every API response SHOULD include `X-Ciel-Served-By: rust\|spring\|dotnet` |
| Backend pin | Clients ignore; debug with request header `X-Ciel-Backend: rust\|spring\|dotnet` |

OpenAPI snapshot: [`../openapi/openapi.yaml`](../openapi/openapi.yaml).  
Parity checklist: [`CONTRACT_PARITY.md`](CONTRACT_PARITY.md).

Implementations live in this repo: Rust (`src/`), Spring (`spring/`), .NET (`dotnet/`).

## Auth (PASETO v4 local)

| Item | Value |
|------|--------|
| Version | PASETO **v4.local** |
| Keys | `PASETO_ACCESS_KEY` / `PASETO_REFRESH_KEY` — base64 of **32 raw bytes** each |
| Issuer (`iss`) | `ciel` |
| Audience (`aud`) | `ciel` |
| Subject (`sub`) | user UUID string |
| Type claim | custom claim **`typ`**: `access` or `refresh` (not JWT `type`) |
| Refresh `jti` | refresh_tokens row UUID |
| Access TTL | `ACCESS_TTL_MINUTES` (default 15) |
| Refresh TTL | `REFRESH_TTL_DAYS` (default 30) |
| Password | Argon2id **PHC** strings (`Argon2::default()` / compatible verify) |
| Refresh storage | SHA-256 hex of refresh token string in `refresh_tokens.token_hash` |
| Reuse | Replaying a rotated refresh token revokes all sessions for that user |

Do **not** switch to JWT for polyglot convenience — that breaks existing clients and opaque multi-impl.

## Redis keys

| Purpose | Key pattern |
|---------|-------------|
| Home feed cache | `feed:home:{userId}` |
| Stories feed cache | `feed:stories:{userId}:{limit}` |
| Presigned URL cache | `presigned:{objectKey}` |
| User rate limit | `ratelimit:{userId}:{action}:{window}` |
| IP rate limit | `ratelimit:ip:{ip}:{action}:{window}` |

Wrong keys under mixed traffic cause stale feeds and broken limits.

## Cursor encoding

```
{rfc3339}/{uuid}
```

Example: `2024-01-15T12:00:00Z/550e8400-e29b-41d4-a716-446655440000`

Parse with `splitn(2, '/')`. Invalid → `400 bad_request`.

## Media & queue

| Item | Contract |
|------|----------|
| Upload object key | `uploads/{owner_id}/{upload_id}.{ext}` |
| Variants | `media/{owner_id}/{upload_id}/thumb.{ext}`, `media/{owner_id}/{upload_id}/medium.{ext}` |
| Thumb / medium | max 200px / 800px (Rust worker) |
| Content types | `image/jpeg`, `image/png`, `image/webp` |
| SQS body (`MediaJob`) | `{"upload_id":"<uuid>","owner_id":"<uuid>","original_key":"<string>"}` |
| Worker ownership | **Rust worker only** — Spring/.NET enqueue jobs, never process pixels |
| Cleanup ownership | **Rust API only** — hourly story/invite/stale-upload cleanup |

## Visibility enums (DB)

Store as text: `public`, `followers_only` (posts); story visibility via existing story enum values.  
JSON for post `visibility` follows Rust serde enum names: `Public`, `FollowersOnly`.

## Process ownership

| Role | Owner |
|------|--------|
| Apply SQL migrations | Rust / migrate job |
| Media processing worker | Rust |
| Hourly cleanup loops | Rust API Deployment only |
| HTTP `/v1` | Rust, Spring, or .NET (any) |

## Env parity

Implementations must honor the same env vars as [`.env.unraid.example`](../.env.unraid.example) for DB, Redis, S3, queue, and PASETO (plus `CIEL_SERVED_BY=rust|spring|dotnet` for the response header).
