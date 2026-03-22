---
title: Getting started
---

PicShare is a photo-only social platform. **Ciel** is the Rust (Axum) backend and media worker; native **iOS** (SwiftUI) and **Android** (Jetpack Compose) apps consume its HTTP API. {% .lead %}

{% quick-links %}

{% quick-link title="Overview" icon="installation" href="/docs/overview/" description="What PicShare is, how the monorepo is organized, and where to read next." /%}

{% quick-link title="Architecture" icon="presets" href="/docs/architecture/" description="How Ciel is layered, how api and worker modes differ, and how data flows." /%}

{% quick-link title="API reference" icon="theming" href="/docs/api-auth/" description="HTTP routes grouped by domain—auth, posts, feed, media, stories, and more." /%}

{% quick-link title="Local development" icon="plugins" href="/docs/local-development/" description="Run the API, worker, and full Docker stack against Postgres, Redis, and LocalStack." /%}

{% /quick-links %}

---

## Who this documentation is for

Contributors and operators who need to understand how the backend and clients fit together, which endpoints exist, and what tradeoffs apply as you scale.

---

## Repository layout (monorepo)

The PicShare monorepo typically contains:

- **Ciel-backend** — API server (`APP_MODE=api`), SQS media worker (`APP_MODE=worker`), SQL migrations, Terraform for Scaleway, and this docs site.
- **PicShare-ios** — SwiftUI app, repositories, use cases, no third-party packages by design.
- **PicShare-android** — Kotlin, Hilt, Retrofit, Room, Compose.

This site focuses on Ciel in depth and summarizes client architecture at a high level.

---

## Quick links to the codebase

- Route definitions: `src/http/routes.rs`
- Handlers: `src/http/handlers.rs`
- Services: `src/app/`
- Domain types: `src/domain/`

---

## Environment

Ciel expects configuration via environment variables (see `.env.example` in the backend repo). Common local workflow: `docker compose up --build` for Postgres, Redis, LocalStack (S3/SQS), API, and worker.
