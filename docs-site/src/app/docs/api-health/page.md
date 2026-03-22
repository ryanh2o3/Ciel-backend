---
title: Health and metrics
---

Base path: **none** (mounted at root of the API router).

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health` | Public | Liveness / readiness style check for load balancers |
| GET | `/metrics` | Varies | Application metrics (see handler for scrape auth if any) |

{% callout title="Note" %}
Exact response bodies are defined in `handlers::health` and `handlers::metrics`. Protect `/metrics` in production if it exposes sensitive counters.
{% /callout %}
