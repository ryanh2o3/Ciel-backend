---
title: Users and account
---

`:id` is a user UUID unless noted.

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/users` | Public* | Create user (registration flow) |
| GET | `/users/:id` | Mixed | Public profile vs extra fields when authenticated |
| PATCH | `/users/:id` | Bearer | Update own profile |
| GET | `/users/:id/posts` | Mixed | User’s posts (visibility rules apply) |
| GET | `/users/:id/stories` | Bearer | User’s active stories |
| GET | `/users/:id/highlights` | Mixed | Story highlights |
| POST | `/users/:id/follow` | Bearer | Follow user |
| POST | `/users/:id/unfollow` | Bearer | Unfollow |
| POST | `/users/:id/block` | Bearer | Block user |
| POST | `/users/:id/unblock` | Bearer | Unblock |
| GET | `/users/:id/followers` | Mixed | Followers list |
| GET | `/users/:id/following` | Mixed | Following list |
| GET | `/users/:id/relationship` | Bearer | Relationship to target user |
| DELETE | `/account` | Bearer | Delete own account |

\*Registration may require a valid invite depending on product configuration.
