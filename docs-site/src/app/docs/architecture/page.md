---
title: System architecture
---

Ciel follows a **layered** layout: each layer depends only on the layer below it.

---

## Backend layers

```text
src/
├── domain/     # Structs (User, Post, Story, …) — no business logic
├── app/        # Services — DB, cache, domain rules
├── http/       # Axum routes, handlers, auth extractors, errors, middleware
├── infra/      # DB pool, Redis, S3, SQS clients
├── config/     # AppConfig from environment
├── jobs/       # Worker jobs (e.g. media_processor)
└── main.rs     # Entry: AppState, api vs worker mode
```

**Handlers** (`http/handlers.rs`) stay thin: parse input, call services, map errors to HTTP. **Services** (`app/`) own transactions, queries, and Redis usage. **Domain** types cross the boundary but contain no I/O.

---

## Request path (API mode)

1. **Router** (`http/mod.rs`) merges route groups from `routes.rs` and applies middleware (rate limits, security, etc.).
2. **Extractors** validate `AuthUser` (Bearer PASETO) or omit auth for public routes.
3. **Handler** invokes `XxxService::new(state.db.clone(), …)` and returns JSON or errors.

---

## Worker mode

The **worker** runs the media pipeline: messages from SQS trigger processing (e.g. transcoding or derivative generation) using the same `AppState`-style infrastructure (DB, object storage) without serving HTTP.

---

## Monorepo context

Ciel Social splits **backend**, **iOS**, and **Android** into sibling projects. Clients talk to Ciel only over HTTPS; they do not share Rust code with the server.
