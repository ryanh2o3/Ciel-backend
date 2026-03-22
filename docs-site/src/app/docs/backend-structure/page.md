---
title: Backend structure
---

## Handlers and routes

- **`src/http/routes.rs`** — All route groups (`health`, `auth`, `users`, `posts`, …). Each function returns a `Router<AppState>`.
- **`src/http/handlers.rs`** — All endpoint handlers in one file (project convention).
- **`src/http/mod.rs`** — Composes the full router and middleware stack.

Path parameters use Axum conventions (`:id` in the route string, `Path(Uuid)` in handlers).

---

## Services

Services are **stateless** `Clone` structs holding a `Db` handle (and optionally `RedisCache`). Example pattern:

```rust
let svc = PostService::new(state.db.clone(), state.redis.clone());
```

They return `anyhow::Result<T>` internally; handlers map failures to `AppError` and HTTP status codes.

---

## Auth

- **PASETO** access tokens (short-lived) and refresh tokens (longer-lived).
- **Argon2** for password hashing.
- **`AuthUser` extractor** — validates `Authorization: Bearer`; 401 if missing or invalid.
- **`AdminToken`** — separate secret for moderation/admin-style endpoints.

---

## Errors

`AppError` in `http/error.rs` centralizes HTTP-facing errors. Handlers often use:

- Database constraint codes (`23505` unique, `23503` foreign key) for 409/404 semantics.
- `anyhow` message checks for domain-level failures exposed as 400/403/404.

---

## Conventions (summary)

| Area | Convention |
|------|------------|
| SQL | `sqlx::query()` with `.bind()`, not `query!` |
| Rows | `row.get("column_name")` |
| UUID PKs | Generated in Postgres (`uuid_generate_v4()`), not in Rust |
| Visibility enums | String `as_db()` / `from_db()`, not sqlx enum derives |
| Cache keys | `namespace:entity:id` |
