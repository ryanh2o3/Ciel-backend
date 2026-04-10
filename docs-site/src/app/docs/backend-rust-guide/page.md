---
title: Backend Rust guide
---

This page is a **guided tour** of the Ciel backend for readers who already know Rust basics (ownership, `Result`, modules, traits) and want to learn how those ideas show up in a real Axum + SQLx service. It is not a replacement for [The Rust Programming Language](https://doc.rust-lang.org/book/)—use the book for language fundamentals.

For system-level behavior (modes, middleware goals, workers), see [System architecture](/docs/architecture/). For a concise map of files and conventions, see [Backend structure](/docs/backend-structure/). For schema and SQL patterns, see [Data and migrations](/docs/data-and-migrations/).

---

## Suggested reading order

Work through the codebase in this order so each layer has context.

1. **`src/main.rs`** — Process entry: `#[tokio::main]`, `AppConfig::from_env()`, connection of `Db`, `RedisCache`, `ObjectStorage`, `QueueClient`, construction of `AppState`, and the `match` on `APP_MODE` (`api`, `worker`, `combined`, `serverless-worker`). Notice background tasks spawned in `api` / `combined` mode (notification worker, cleanup loop) and graceful shutdown with `CancellationToken`.
2. **`src/lib.rs`** — Crate root: module tree and `AppState`. This struct is the **composition root** for everything handlers and middleware need; it is `Clone` so Axum can share it cheaply across concurrent requests.
3. **`src/config/`** — How environment variables become `AppConfig` (startup fails fast on bad keys or URLs).
4. **`src/http/mod.rs`** and **`src/http/routes.rs`** — How the `Router` is assembled: `nest("/v1", …)`, `merge` of route groups, per-group `layer` chains (IP rate limit, user rate limit, ban check), then global layers (metrics, CORS, request IDs, request context, security, compression, body limit). See [System architecture](/docs/architecture/) for the request lifecycle diagram.
5. **Auth vertical slice** — `src/http/auth.rs` (`AuthUser`, `AdminToken` extractors), `src/app/auth.rs` (`AuthService`), and the matching handlers in `src/http/handlers.rs` / routes in `routes.rs`. This shows `FromRequestParts<AppState>`, mapping service errors to `AppError`, and PASETO validation without putting crypto in handlers.
6. **Data-heavy slice** — `src/app/feed.rs` (or `posts.rs`): SQLx queries, optional Redis in `src/infra/cache.rs`, cursor pagination, and plain structs from `src/domain/` returned through handlers.

After that, explore **`src/jobs/media_processor.rs`** for async background work and **`src/http/error.rs`** for how HTTP statuses are centralized.

---

## Rust concepts in this codebase

### Async runtime and `#[tokio::main]`

The binary is async end-to-end: `main` is `async`, the HTTP server and workers `.await` I/O. Background work uses `tokio::spawn` (for example the media worker in `combined` mode, or API-side notification/cleanup tasks). Shutdown uses `tokio::signal` and `CancellationToken` so in-flight work can drain cooperatively.

### `Clone` on `AppState`, `Db`, and services

`AppState` derives `Clone`. The database pool and other infra handles are cheap to clone (they are internally shared). Axum passes `State<AppState>` into handlers and middleware; the router builder in `http/mod.rs` calls `state.clone()` many times so each middleware closure owns a copy. That pattern avoids global singletons and keeps the type system explicit about what each layer can access.

### `Arc` for shared immutable configuration

Values that must be shared across tasks without cloning large data use `Arc`, for example `trusted_proxy_cidrs` on `AppState`. The contents are read-only after startup; concurrent handlers only need immutable access.

### `mpsc` and notification jobs

API mode creates an `mpsc::channel` for `NotificationJob` and stores the sender in `AppState`. Handlers enqueue work; a dedicated task in `src/jobs/notifications.rs` consumes messages and talks to the database. This decouples “request completed” from “notification row written” without blocking the HTTP response path for all of that work.

### `anyhow::Result` in services vs `AppError` at the edge

Services under `src/app/` return `anyhow::Result<T>` for flexibility and rich context. Handlers translate failures into `AppError` (and thus HTTP status + stable client-facing messages). Extractors like `AuthUser` also use `AppError` as `Rejection`. That split keeps business logic from depending on HTTP types while preserving one place (`error.rs`) for response mapping.

### Axum extractors: `State`, `FromRequestParts`, JSON

- **`State<AppState>`** — Injects the shared application state.
- **`AuthUser` / `AdminToken`** — Implement `FromRequestParts<AppState>` in `http/auth.rs`: async parsing of headers, optional DB call, and `Result<_, AppError>`.
- **JSON bodies** — Typed deserialization with serde; validation helpers live in `http/validation.rs` (length rules, trimmed strings, etc.).

### `IntoResponse` and error handling

`AppError` implements conversion to HTTP responses so handlers can return `Result<impl IntoResponse, AppError>` or use `?` with types that compose cleanly with the error type in use.

---

## Trace-through 1: Authenticated JSON request

1. A client sends `GET` or `POST` to `/v1/...` with `Authorization: Bearer …`.
2. Global middleware runs (metrics, CORS, request IDs, trusted-proxy-aware IP/scheme in `middleware/request_context`, security headers, compression, body size cap)—see [System architecture](/docs/architecture/).
3. The matched route group runs its layers (IP and/or per-user rate limits, ban check where configured).
4. Axum runs extractors: for protected routes, `AuthUser` runs first. It reads the bearer token, builds `AuthService::new(state.db.clone(), …)`, calls `authenticate_access_token`, and yields `AuthUser { user_id }` or `401`.
5. The handler constructs one or more `*Service::new(state.db.clone(), …)` (and cache/storage as needed), calls async service methods, and builds JSON responses.
6. On SQL errors, the handler inspects `sqlx::Error` / constraint codes when needed and maps to `AppError` (see [Backend structure](/docs/backend-structure/)).

Useful API docs for the flow: [Auth and invites](/docs/api-auth/), [Users and account](/docs/api-users/).

---

## Trace-through 2: Media upload and processing

1. The client uses the media API (see [Media](/docs/api-media/)) to obtain upload instructions, then uploads bytes to object storage.
2. The API records upload state in Postgres and sends a message to the SQS-compatible queue via `QueueClient`.
3. In `worker` mode (or the spawned loop in `combined`), `src/jobs/media_processor.rs` polls the queue, loads job metadata from the DB, fetches the object from storage, generates derivatives (e.g. thumb/medium), writes new keys, and updates media rows.
4. Processing uses **status transitions** so retries are safe: duplicate deliveries observe “already completed” (or similar) and do not corrupt data. Permanent failures mark the row failed and acknowledge the message; transient failures leave the message for redelivery.

This matches the idempotency rules described in [System architecture](/docs/architecture/) and ties together [Platform components](/docs/platform-components/) (S3 + SQS + Postgres).

---

## Where to go next

| Topic | Doc |
|--------|-----|
| Layers and diagrams | [System architecture](/docs/architecture/) |
| File layout and SQL/cache conventions | [Backend structure](/docs/backend-structure/) |
| Tables and migrations | [Data and migrations](/docs/data-and-migrations/) |
| HTTP routes by area | [API reference](/docs/api-health/) (and sibling pages) |

When you change behavior, update the Rust code first; these docs should stay aligned with `src/` layout and `APP_MODE` semantics in `main.rs`.
