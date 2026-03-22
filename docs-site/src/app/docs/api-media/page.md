---
title: Media
---

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/media/upload` | Bearer | Start upload — returns upload id / URL instructions |
| POST | `/media/upload/:id/complete` | Bearer | Mark upload complete; may enqueue processing |
| GET | `/media/upload/:id/status` | Bearer | Processing status |
| GET | `/media/:id` | Mixed | Metadata or redirect policy per deployment |
| DELETE | `/media/:id` | Bearer | Remove media (permissions apply) |

Processing is asynchronous: the **worker** consumes queue messages and writes derivatives to object storage.
