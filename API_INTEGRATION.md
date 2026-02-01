## PicShare API Integration Guide

This document describes how a client app connects to the PicShare backend, how to use auth, and how to call every endpoint. It also includes data access and pagination best practices for scalable clients.

### Base URL
- Local Docker Compose: `http://localhost:8080`

### Authentication Overview (PASETO v4 local)
- Access token: short-lived, sent on every authenticated request.
- Refresh token: long-lived, used to mint new access tokens.
- Header for authenticated calls: `Authorization: Bearer <access_token>`

Best practices:
- Keep access tokens in memory (or OS secure storage) and refresh on 401.
- Never log tokens or store them in plaintext in analytics.
- Rotate refresh tokens on each refresh and replace stored token with the new value.

### Error Format
All errors are JSON:
```json
{ "error": "message" }
```

### Pagination
All list endpoints use:
- `limit` (default 30, max 200)
- `cursor` (opaque string)

Response includes:
```json
{
  "items": [],
  "next_cursor": "..."
}
```

Treat `next_cursor` as an opaque token. Pass it back on the next request to continue where you left off.

### Common Resource Shapes

**User**
```json
{
  "id": "uuid",
  "handle": "string",
  "email": "string",
  "display_name": "string",
  "bio": "string|null",
  "avatar_key": "string|null",
  "created_at": "RFC3339"
}
```

**Post**
```json
{
  "id": "uuid",
  "owner_id": "uuid",
  "media_id": "uuid",
  "caption": "string|null",
  "visibility": "public|followers_only",
  "created_at": "RFC3339"
}
```

**Media**
```json
{
  "id": "uuid",
  "owner_id": "uuid",
  "original_key": "string",
  "thumb_key": "string",
  "medium_key": "string",
  "width": 0,
  "height": 0,
  "bytes": 0,
  "created_at": "RFC3339"
}
```

**Like**
```json
{
  "id": "uuid",
  "user_id": "uuid",
  "post_id": "uuid",
  "created_at": "RFC3339"
}
```

**Comment**
```json
{
  "id": "uuid",
  "user_id": "uuid",
  "post_id": "uuid",
  "body": "string",
  "created_at": "RFC3339"
}
```

**Notification**
```json
{
  "id": "uuid",
  "user_id": "uuid",
  "notification_type": "string",
  "payload": {},
  "read_at": "RFC3339|null",
  "created_at": "RFC3339"
}
```

**ModerationAction**
```json
{
  "id": "uuid",
  "actor_id": "uuid",
  "target_type": "string",
  "target_id": "uuid",
  "reason": "string|null",
  "created_at": "RFC3339"
}
```

### Auth Endpoints

**POST `/auth/login`**
- Body:
```json
{ "email": "demo@example.com", "password": "ChangeMe123!" }
```
- Response:
```json
{
  "access_token": "string",
  "refresh_token": "string",
  "access_expires_at": "RFC3339",
  "refresh_expires_at": "RFC3339"
}
```

**POST `/auth/refresh`**
- Body:
```json
{ "refresh_token": "string" }
```
- Response: same as `/auth/login` with rotated refresh token.

**POST `/auth/revoke`**
- Body:
```json
{ "refresh_token": "string" }
```
- Response: `204 No Content` on success.

**GET `/auth/me`** (auth required)
- Response: `User`

### Users & Profiles

**POST `/users`** (signup)
- Body:
```json
{
  "handle": "demo",
  "email": "demo@example.com",
  "display_name": "Demo User",
  "bio": "optional",
  "avatar_key": "optional",
  "password": "ChangeMe123!"
}
```
- Response: `User`

**GET `/users/:id`**
- Response: `User`

**PATCH `/users/:id`** (auth required, only self)
- Body:
```json
{ "display_name": "New Name", "bio": "New bio", "avatar_key": "optional" }
```
- Response: `User`

**GET `/users/:id/posts`**
- Query: `limit`, `cursor`
- Response: `ListResponse<Post>`

### Social Graph (auth required)

**POST `/users/:id/follow`**
- Response:
```json
{ "followed": true }
```

**POST `/users/:id/unfollow`**
- Response:
```json
{ "unfollowed": true }
```

**POST `/users/:id/block`**
- Response:
```json
{ "blocked": true }
```

**POST `/users/:id/unblock`**
- Response:
```json
{ "unblocked": true }
```

**GET `/users/:id/followers`**
- Query: `limit`, `cursor`
- Response:
```json
{
  "items": [ { "user": { ...User }, "followed_at": "RFC3339" } ],
  "next_cursor": "..."
}
```

**GET `/users/:id/following`**
- Same response shape as followers.

**GET `/users/:id/relationship`**
- Response:
```json
{
  "is_following": false,
  "is_followed_by": false,
  "is_blocking": false,
  "is_blocked_by": false
}
```

### Posts

**POST `/posts`** (auth required)
- Body:
```json
{ "media_id": "uuid", "caption": "optional" }
```
- Response: `Post`

**GET `/posts/:id`**
- Public if visibility allows, otherwise requires auth.
- Response: `Post`

**PATCH `/posts/:id`** (auth required, owner only)
- Body:
```json
{ "caption": "New caption" }
```
- Response: `Post`

**DELETE `/posts/:id`** (auth required, owner only)
- Response: `204 No Content`

### Engagement

**POST `/posts/:id/like`** (auth required)
- Response:
```json
{ "created": true }
```

**DELETE `/posts/:id/like`** (auth required)
- Response: `204 No Content`

**GET `/posts/:id/likes`**
- Query: `limit`, `cursor`
- Response: `ListResponse<Like>`

**POST `/posts/:id/comment`** (auth required)
- Body:
```json
{ "body": "Nice shot!" }
```
- Response: `Comment`

**GET `/posts/:id/comments`**
- Query: `limit`, `cursor`
- Response: `ListResponse<Comment>`

**DELETE `/posts/:id/comments/:comment_id`** (auth required, author only)
- Response: `204 No Content`

### Feed (auth required)

**GET `/feed`**
- Query: `limit`, `cursor`
- Response: `ListResponse<Post>`

**POST `/feed/refresh`**
- Response: `204 No Content`

### Media

**POST `/media/upload`** (auth required)
- Body:
```json
{ "content_type": "image/jpeg", "bytes": 12345 }
```
- Response:
```json
{
  "upload_id": "uuid",
  "object_key": "string",
  "upload_url": "string",
  "expires_in_seconds": 900,
  "headers": [ { "name": "string", "value": "string" } ]
}
```

**Client upload step (direct to object storage)**
- Perform an HTTP `PUT` to `upload_url`.
- Include any headers returned in `headers`.
- Send the raw image bytes.

**POST `/media/upload/:id/complete`** (auth required)
- Response: `202 Accepted` when processing is queued.

**GET `/media/upload/:id/status`** (auth required)
- Response:
```json
{ "status": "pending|uploaded|processing|failed|completed", "processed_media_id": "uuid|null" }
```

**GET `/media/:id`**
- Response: `Media`

**DELETE `/media/:id`** (auth required, owner only)
- Response: `204 No Content`

### Notifications (auth required)

**GET `/notifications`**
- Query: `limit`, `cursor`
- Response: `ListResponse<Notification>`

**POST `/notifications/:id/read`**
- Response: `204 No Content`

### Moderation (auth required)

**POST `/moderation/users/:id/flag`**
- Body:
```json
{ "reason": "optional" }
```
- Response: `UserFlag`

**POST `/moderation/posts/:id/takedown`**
- Body:
```json
{ "reason": "optional" }
```
- Response: `204 No Content`

**POST `/moderation/comments/:id/takedown`**
- Body:
```json
{ "reason": "optional" }
```
- Response: `204 No Content`

**GET `/moderation/audit`**
- Query: `limit`, `cursor`
- Response: `ListResponse<ModerationAction>`

### Search/Discovery

**GET `/search/users?q=...`**
- Query: `q` (min 2 chars), `limit`, `cursor`
- Response: `ListResponse<User>`

**GET `/search/posts?q=...`**
- Query: `q` (min 2 chars), `limit`, `cursor`
- Response: `ListResponse<Post>`

### Client Data Access Patterns (Best Practices)
- Cache the home feed for short intervals and rely on `next_cursor` to continue paging.
- Always request posts and comments in descending order (the API returns newest first).
- De-duplicate list results by ID when mixing pagination with local updates.
- For media: treat `Media` records as metadata; render images via CDN/object storage using `*_key` fields.
- Prefer optimistic UI updates for likes/comments, then reconcile with the API response.
- Use `POST /feed/refresh` after heavy activity (posting, following) to invalidate cache.
- Keep refresh tokens secure and rotate on each refresh call.

