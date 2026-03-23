---
title: Scaling
---

Practical levers that match Ciel’s architecture.

---

## Horizontal API instances

Run multiple **API** processes behind a load balancer. Requirements:

- **Stateless** HTTP layer (session state in tokens + DB/Redis).
- **Sticky sessions** only if you intentionally rely on them (default Scaleway config in Terraform may use cookie stickiness—validate for your release).
- Keep all instances on the same migration version before enabling traffic.

---

## Postgres

- **Connection pools** per instance; size pools so total connections stay under DB limits.
- Tune through env vars: `DB_MAX_CONNECTIONS`, `DB_CONNECT_TIMEOUT_SECONDS`, `DB_IDLE_TIMEOUT_SECONDS`, `DB_MAX_LIFETIME_SECONDS`.
- Keep `sum(instance_pools)` under server limits with headroom for admin tools and background tasks.
- Read replicas can help analytics/reporting workloads, but core API paths currently assume primary write/read consistency.

---

## Redis

- Used as a **cache**, not the source of truth. Safe to scale memory/eviction policies as traffic grows.
- Feed cache TTL is intentionally short (currently 30s) to balance freshness with burst absorption.
- **Cache-miss resilience**: services tolerate Redis errors and fall through to Postgres.
- Ensure eviction policy aligns with short-lived keys (avoid retaining stale hot keys indefinitely).

---

## Object storage and CDN

- Serve **processed** images from a **CDN** (e.g. Scaleway Edge Services) in front of the bucket to cut origin load and latency.
- **Presigned uploads** keep upload traffic off the API body size limits.
- Favor immutable media object paths for high CDN hit ratios and safe long cache lifetimes.

---

## Queue and workers

- Scale **worker** concurrency (process count or container replicas) with queue depth and S3 throughput.
- Use a **DLQ** for poison messages so retries do not block the main queue.
- Worker processing is idempotent via DB status checks; this supports at-least-once queue delivery.
- Separate permanent failures (unsupported/invalid media) from transient infra errors to prevent retry storms.

---

## Rate limiting

Middleware enforces configurable limits (see `config` and `http/middleware`). Tune per route class (auth vs read vs write) under attack or growth.

- **IP limits** protect unauthenticated endpoints (`/auth/login`, signup, health).
- **Trust-level limits** protect authenticated actions and can scale by account maturity.
- Monitor `429` rates by endpoint/action before increasing limits globally.

---

## Process modes and deployment shape

- Run `APP_MODE=api` for web-serving instances.
- Run `APP_MODE=worker` for dedicated queue consumers.
- Use `APP_MODE=combined` only when operational simplicity matters more than fault isolation.
- For event-driven/serverless ingestion, `APP_MODE=serverless-worker` exposes a minimal HTTP handler that executes a single media job payload.

---

## Observability and safe rollouts

- Expose `/metrics` and ingest request/latency/error and queue depth metrics.
- Track feed cache hit rate, media-job success/failure mix, and DB pool saturation.
- Roll out with canaries when changing middleware behavior, queue logic, or schema-heavy features.
