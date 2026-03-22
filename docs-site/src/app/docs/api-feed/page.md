---
title: Feed
---

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/feed` | Bearer | Home feed (cursor pagination) |
| POST | `/feed/refresh` | Bearer | Invalidate or rebuild cached feed snapshot |

Feed entries may be backed by **Redis** for hot paths; see [Scaling](/docs/scaling/).
