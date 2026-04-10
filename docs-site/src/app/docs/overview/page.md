---
title: Overview
---

Ciel Social is a **photo-only** social product: posts and stories revolve around images, with likes, comments, follows, notifications, search, moderation, and an invite system.

---

## Ciel (backend)

**Ciel** is the backend codename. It is written in **Rust** with **Axum**, **SQLx** (Postgres), **Redis**, S3-compatible **object storage**, and **SQS**-compatible queues for asynchronous media processing.

Two runtime modes share one codebase:

| Mode | Purpose |
|------|---------|
| `APP_MODE=api` | Public HTTP API on `HTTP_ADDR` (default `0.0.0.0:8080`) |
| `APP_MODE=worker` | Consumes upload/processing messages from the queue |

---

## Clients

- **iOS** — SwiftUI, Clean Architecture–style layers (Domain, Data, Features, Core).
- **Android** — Jetpack Compose, Hilt, Retrofit, Room; repository pattern aligned with use cases.

Both apps authenticate with **Bearer** access tokens (PASETO) and refresh tokens as implemented by Ciel.

---

## Documentation map

- **Architecture** — Layers, modes, middleware order, background tasks, and media/notification flows (with diagrams).
- **Backend structure** — Where handlers, services, jobs, and infra modules live; SQL and cache conventions.
- **Backend Rust guide** — Reading order and how Rust patterns (`async`, `Clone`, extractors, errors) show up in this codebase.
- **Platform components** — Postgres, Redis, object storage, queues, load balancer.
- **API reference** — Tables of routes aligned with `src/http/routes.rs`.
- **Scaling / limitations** — Caching, pagination, and pragmatic caps.
