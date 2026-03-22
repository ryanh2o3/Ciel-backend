---
title: Stories
---

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/stories` | Bearer | Create story |
| GET | `/stories/:id` | Mixed | Get story |
| DELETE | `/stories/:id` | Bearer | Delete story |
| GET | `/stories/:id/viewers` | Bearer | Viewers |
| POST | `/stories/:id/reactions` | Bearer | Add reaction |
| GET | `/stories/:id/reactions` | Mixed | List reactions |
| DELETE | `/stories/:id/reactions` | Bearer | Remove reaction |
| POST | `/stories/:id/seen` | Bearer | Mark seen |
| GET | `/stories/:id/metrics` | Bearer | Aggregated metrics |
| POST | `/stories/:id/highlights` | Bearer | Add to highlight reel |
| GET | `/feed/stories` | Bearer | Stories feed |

Stories expire after a short TTL (product default, e.g. 24h); cleanup jobs remove stale rows.
