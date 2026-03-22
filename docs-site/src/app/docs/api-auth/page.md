---
title: Auth and invites
---

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/auth/login` | Public | Email/password → tokens |
| POST | `/auth/refresh` | Public | Refresh token → new access (and possibly refresh) |
| POST | `/auth/revoke` | Bearer | Revoke refresh / session |
| GET | `/auth/me` | Bearer | Current user profile |
| GET | `/invites/validate/:code` | Public | Validate invite code before signup |

Tokens are **PASETO**; send access token as `Authorization: Bearer <token>` on protected routes.
