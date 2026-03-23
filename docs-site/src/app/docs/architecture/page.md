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

### Architectural decisions

- **Thin HTTP boundary**: handlers in `http/handlers.rs` parse/validate requests, call services, and map failures into `AppError`.
- **Service-centric business logic**: `app/` services own query composition, transaction boundaries, cache interaction, and domain rules.
- **Domain types as transport objects**: `domain/` structs are intentionally light and free of I/O concerns.
- **Infrastructure adapters isolated in `infra/`**: Postgres, Redis, queue, and object-storage clients are wrapped so service code depends on stable interfaces.

---

## Runtime modes

`APP_MODE` controls process behavior from a single binary:

| Mode | What runs |
|------|-----------|
| `api` | HTTP API + in-process notification worker + cleanup loop |
| `worker` | SQS media processing loop only |
| `combined` | API and media worker in one process (useful in smaller environments) |
| `serverless-worker` | Minimal HTTP endpoint that executes one media job payload per request |

This split keeps the API path responsive while allowing background work to scale independently.

---

## Request path (API mode)

1. **Router assembly** (`http/mod.rs`) merges route groups from `routes.rs` and nests them under `/v1` (except `/health` and `/metrics`).
2. **Global middleware** is applied for request IDs, proxy-aware request context, security headers/HTTPS handling, compression, and body limits.
3. **Per-route-group middleware** adds auth-aware rate limiting and ban checks where required.
4. **Extractors** validate `AuthUser` (Bearer PASETO) or `AdminToken` for privileged routes.
5. **Handler/service flow** creates service instances from `AppState` clones and returns JSON or mapped HTTP errors.

---

## Middleware model

The middleware stack is intentionally layered to protect correctness at scale:

- **Request context first**: trusted proxy CIDRs determine whether `X-Forwarded-For` and `X-Forwarded-Proto` are honored.
- **Security middleware** relies on resolved scheme from request context, avoiding blind trust of forwarded headers from untrusted peers.
- **Rate limits** are split:
  - IP-based limits for unauthenticated/high-risk entry points (`/auth/login`, `/users`, `/health`).
  - User/trust-level limits for authenticated actions (`post`, `like`, `comment`, `feed`, `search`, `media_*`, moderation).
- **Metrics and request IDs** provide traceability and Prometheus-ready observability.

---

## Worker mode

The media worker is built for at-least-once delivery semantics:

- Polls SQS-compatible queues and processes one job at a time in a loop.
- Uses DB state transitions (`uploaded -> processing -> completed/failed`) to make retries idempotent.
- Classifies errors as transient vs permanent:
  - **Permanent** (e.g. unsupported image type/decode failure): mark failed and consume message.
  - **Transient** (network/storage/DB blips): keep message for retry.
- Produces derivatives (`thumb`, `medium`) and writes metadata back to Postgres.

In `api` mode, notification and cleanup background jobs are also managed with graceful shutdown.

---

## Monorepo context

Ciel Social splits **backend**, **iOS**, and **Android** into sibling projects. Clients talk to Ciel only over HTTPS; they do not share Rust code with the server.
