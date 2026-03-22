---
title: Scaling
---

Practical levers that match Ciel’s architecture.

---

## Horizontal API instances

Run multiple **API** processes behind a load balancer. Requirements:

- **Stateless** HTTP layer (session state in tokens + DB/Redis).
- **Sticky sessions** only if you intentionally rely on them (default Scaleway config in Terraform may use cookie stickiness—validate for your release).

---

## Postgres

- **Connection pools** per instance; size pools so total connections stay under DB limits.
- **Read replicas** (if enabled in your environment) for read-heavy reporting—not all paths may use them yet; verify service code.

---

## Redis

- Used as a **cache**, not the source of truth. Safe to scale memory/eviction policies as traffic grows.
- **TTLs** limit stale feed risk; `POST /feed/refresh` supports explicit invalidation.

---

## Object storage and CDN

- Serve **processed** images from a **CDN** (e.g. Scaleway Edge Services) in front of the bucket to cut origin load and latency.
- **Presigned uploads** keep upload traffic off the API body size limits.

---

## Queue and workers

- Scale **worker** concurrency (process count or container replicas) with queue depth and S3 throughput.
- Use a **DLQ** for poison messages so retries do not block the main queue.

---

## Rate limiting

Middleware enforces configurable limits (see `config` and `http/middleware`). Tune per route class (auth vs read vs write) under attack or growth.
