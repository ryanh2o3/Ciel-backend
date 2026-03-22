---
title: Limitations
---

Honest constraints to plan around.

---

## No GraphQL / subscriptions

The public contract is **REST/JSON** over HTTP. Real-time features (if any) are not documented here as first-class WebSocket APIs unless added explicitly to `routes.rs`.

---

## Photo-first product scope

Features are centered on **photos**—do not assume video-first or arbitrary file types without checking handlers and storage policies.

---

## Worker latency

Media appears **asynchronous**: uploads complete before all derivatives exist. Clients should poll **upload status** or use product-specific UX.

---

## Cache staleness

Redis-cached feeds can be **briefly stale** relative to Postgres. Refresh endpoints and TTLs mitigate but do not guarantee read-after-write consistency across all reads.

---

## Operational coupling

**Migrations** must run before code that depends on new columns. **Feature flags** in clients should align with backend deployment order.

---

## Documentation vs code

This site is **hand-written** from the codebase. If `routes.rs` and these tables diverge, **the code wins** until docs are updated. A future OpenAPI spec could narrow that gap.
