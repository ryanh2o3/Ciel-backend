---
title: Moderation and admin
---

Moderation routes use the **`AdminToken`** (or equivalent) extractor — not end-user Bearer tokens. Configure `ADMIN_TOKEN` (or project-specific secret) in the environment.

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/moderation/users/:id/flag` | Admin | Flag user for review |
| POST | `/moderation/posts/:id/takedown` | Admin | Takedown post |
| POST | `/moderation/comments/:id/takedown` | Admin | Takedown comment |
| GET | `/moderation/audit` | Admin | Audit log |
| POST | `/admin/invites` | Admin | Create invite codes |

{% callout type="warning" title="Security" %}
Never expose the admin token to clients. Use server-side tools or secured operator consoles only.
{% /callout %}
