---
title: Local development
---

## Backend (Ciel)

From the **Ciel-backend** repository root:

```bash
cargo build
cargo check
APP_MODE=api cargo run      # HTTP API
APP_MODE=worker cargo run   # Media worker (needs queue + storage)
```

Use a `.env` file or export variables matching `.env.example` (database URL, Redis, S3, queue, PASETO keys, etc.).

---

## Full stack with Docker

```bash
docker compose up --build
```

Brings up Postgres, Redis, LocalStack (S3 + SQS), the API, and the worker. Migrations run as part of the stack.

Optional seeding:

```bash
bash docker/seed/seed.sh
bash docker/seed/upload_media.sh
```

Default demo credentials after seed (if unchanged): `demo@example.com` / `ChangeMe123!`

---

## Health check

```bash
curl -s http://localhost:8080/health
```

---

## Clients

- **iOS** — Build in Xcode; point the app at your local API base URL.
- **Android** — `./gradlew assembleDebug`; configure base URL for your machine or emulator networking.

---

## This docs site

```bash
cd docs-site
npm install
npm run dev
```

Static export for hosting: `npm run build` produces the `out/` directory (see [Docs deployment](/docs/docs-deployment/)).
