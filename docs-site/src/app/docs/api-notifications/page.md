---
title: Notifications
---

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/notifications` | Bearer | List notifications (paginated) |
| POST | `/notifications/:id/read` | Bearer | Mark single notification read |

Payload shapes follow domain types serialized in handlers (JSON).
