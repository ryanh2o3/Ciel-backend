---
title: Safety and invites
---

Trust, device registration, rate-limit introspection, and authenticated invite management.

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/account/trust-score` | Bearer | Trust score for current user |
| GET | `/account/rate-limits` | Bearer | Effective rate limits |
| POST | `/account/device/register` | Bearer | Register device fingerprint |
| GET | `/account/devices` | Bearer | List registered devices |
| GET | `/invites` | Bearer | List invites created by user |
| POST | `/invites` | Bearer | Create invite |
| GET | `/invites/stats` | Bearer | Invite usage stats |
| POST | `/invites/:code/revoke` | Bearer | Revoke invite |

Public invite **validation** lives under [Auth and invites](/docs/api-auth/) (`/invites/validate/:code`).
