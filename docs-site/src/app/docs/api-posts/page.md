---
title: Posts
---

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/posts` | Bearer | Create photo post |
| GET | `/posts/:id` | Mixed | Get post (visibility) |
| PATCH | `/posts/:id` | Bearer | Update caption / metadata |
| DELETE | `/posts/:id` | Bearer | Delete post |
| POST | `/posts/:id/like` | Bearer | Like |
| DELETE | `/posts/:id/like` | Bearer | Unlike |
| GET | `/posts/:id/likes` | Mixed | List likes |
| POST | `/posts/:id/comment` | Bearer | Add comment |
| GET | `/posts/:id/comments` | Mixed | List comments |
| DELETE | `/posts/:id/comments/:comment_id` | Bearer | Delete comment (owner/moderation rules) |

Multi-image posts follow the same resource model with ordered media attachments (see migrations and handlers for limits).
