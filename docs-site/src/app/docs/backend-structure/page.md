---
title: Backend structure
---

For a guided tour of the Rust code (reading order, `Clone`/`Arc`/async patterns, trace-throughs), see [Backend Rust guide](/docs/backend-rust-guide/).

---

## Handlers and routes

- **`src/http/routes.rs`** — All route groups (`health`, `auth`, `users`, `posts`, …). Each function returns a `Router<AppState>`.
- **`src/http/handlers.rs`** — All endpoint handlers in one file (project convention).
- **`src/http/mod.rs`** — Composes the full router and middleware stack.

Path parameters use Axum conventions (`:id` in the route string, `Path(Uuid)` in handlers).

Versioning convention:

- `/health` and `/metrics` live at root.
- Product APIs are nested under `/v1/...`.

---

## `src/app/` services (module map)

Each file under `src/app/` is a **service module** (business logic + SQL/cache). Declared in `src/app/mod.rs`:

| Module | Role |
|--------|------|
| `auth` | Login, registration, token issue/refresh, access-token validation used by HTTP extractors |
| `users` | Profiles, account settings, user lookup |
| `posts` | Photo posts, captions, visibility |
| `feed` | Home and related feeds, Redis-backed caching, cursor pagination |
| `engagement` | Likes, comments, reactions |
| `social` | Follow graph, relationships |
| `media` | Upload flow, presigned URLs, enqueue processing |
| `stories` | Ephemeral stories |
| `search` | Search queries over public data |
| `notifications` | Create and list notifications; `jobs::notifications` invokes this service from the `mpsc` worker |
| `invites` | Invite codes and redemption |
| `moderation` | Reports and moderation actions |
| `trust` | Trust / safety-related user state |
| `fingerprint` | Client fingerprinting helpers for abuse signals |
| `rate_limiter` | Rate-limit bookkeeping backed by Redis/DB as implemented |

---

## `src/jobs/`

Long-running or asynchronous work outside the request hot path:

| Module | Role |
|--------|------|
| `media_processor` | SQS poll loop (or `combined` spawn), derivatives, idempotent DB updates |
| `notifications` | Consumes `NotificationJob` from `mpsc`, writes notification-related rows |
| `cleanup` | Periodic tasks (e.g. expired stories) |

`APP_MODE=serverless-worker` in `main.rs` exposes a small **HTTP** router from `jobs::media_processor` for one-shot processing in serverless environments—separate from the main `http::router` stack.

---

## `src/infra/`

Adapters for external systems; constructed at startup and held on `AppState`:

| Module | Type | Typical use |
|--------|------|-------------|
| `db` | `Db` (SQLx pool) | All persistent state |
| `cache` | `RedisCache` | Feed and rate-limit acceleration |
| `storage` | `ObjectStorage` | S3-compatible uploads and processed keys |
| `queue` | `QueueClient` | Enqueue media jobs for workers |

Services take `Db` / cache handles via `::new` rather than calling infra modules directly from handlers.

---

## `src/http/validation.rs`

Shared **input validation** helpers: max lengths, trimmed required strings, handle format rules. Handlers return `AppError::bad_request(...)` when validation fails, keeping rules in one place instead of duplicating checks across `handlers.rs`.

---

## Services

Services are **stateless** `Clone` structs holding a `Db` handle (and optionally `RedisCache`). Example pattern:

```rust
let svc = PostService::new(state.db.clone(), state.redis.clone());
```

They return `anyhow::Result<T>` internally; handlers map failures to `AppError` and HTTP status codes.

### Why this pattern

- **No request-local mutable service state** keeps handlers easy to reason about and safe to clone.
- **Explicit constructor dependencies** make service wiring visible (`Db`, cache, storage) instead of hidden globals.
- **Failure mapping at the edge** preserves rich internal errors while providing stable API semantics.

---

## Auth

- **PASETO** access tokens (short-lived) and refresh tokens (longer-lived).
- **Argon2** for password hashing.
- **`AuthUser` extractor** — validates `Authorization: Bearer`; 401 if missing or invalid.
- **`AdminToken`** — separate secret for moderation/admin-style endpoints.

Token keys are loaded from environment as 32-byte base64-decoded values; startup fails fast when malformed.

---

## Errors

`AppError` in `http/error.rs` centralizes HTTP-facing errors. Handlers often use:

- Database constraint codes (`23505` unique, `23503` foreign key) for 409/404 semantics.
- `anyhow` message checks for domain-level failures exposed as 400/403/404.

This keeps SQL/infra details out of response payloads while still making errors actionable in logs.

---

## Query and data patterns

### SQL conventions

| Area | Convention |
|------|------------|
| SQL API | `sqlx::query()` with `.bind()`, not `query!` |
| Row access | `row.get("column_name")` via `sqlx::Row` |
| UUID PKs | Generated in Postgres (`uuid_generate_v4()`), not in Rust |
| Enums | Bound as strings via `as_db()` / parsed with `from_db()` |
| Pagination | Cursor `(timestamp, uuid)` with `limit + 1` |

### Cache conventions

- Redis is used for acceleration, never as source of truth.
- Keys follow `namespace:entity:id` (for paged feeds, cursor values are part of the key).
- Cache failures are logged and tolerated; DB remains the fallback path.

### Background processing conventions

- Queue consumers are idempotent against DB status transitions.
- Message deletion happens only after successful or permanently failed processing.
- Transient failures are retried using queue semantics, not custom retry tables.

---

## AppState boundary

`AppState` is defined in `src/lib.rs` and **constructed** in `src/main.rs`—the composition root for runtime dependencies:

- `Db`, `RedisCache`, `ObjectStorage`, `QueueClient`
- auth/token settings and TTLs
- upload constraints (`upload_max_bytes`, upload URL TTL)
- trusted proxy CIDRs and notification channel capacity

Centralizing this boundary keeps handler and middleware construction uniform across API and worker-adjacent paths.
