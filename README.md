# PicShare (Work in progress - under active development)

PicShare is a photo‑only social media app focused on simple sharing and a clean feed. It is designed to run centrally in Europe with a focus on efficiency, low operating costs, and predictable scaling.

**License**: AGPL-3.0 - See [LICENSE](LICENSE) file for details.

This has not been deployed to cloud services yet. Still working on the ios and android clients. Don't want to be costing a fortune just for me to test my half finished clients :)

## Goals

- Simple, fast photo sharing with a clean feed
- Efficient media handling using object storage and CDN
- Strong cache usage to keep infrastructure costs low
- Architecture that scales without major rewrites
- Eventually allow for e2e encryption option for users

## Scope (backend)

- Accounts and profiles
- Follow graph
- Photo posts
- Likes and comments
- Home feed
- Media upload and processing pipeline

## Scale-ready architecture

PicShare is designed to be cost-effective at launch while keeping the same core architecture as it scales to millions of users. The system stays a modular monolith for simplicity, but with clear async boundaries so each worker can scale independently without a rearchitecture.

**Key principles**

- Keep synchronous APIs for user-facing reads/writes; move heavy work to async jobs.
- Use event-driven workflows for media processing, notifications, and feed hydration.
- Add batch/interval jobs for smoothing traffic spikes and periodic backfills.
- Rely on object storage + CDN so the API never serves binary media.

**Scaleway-first building blocks**

- **Compute**: start with 1–2 small instances; scale horizontally for API and workers.
- **Database**: managed PostgreSQL; add read replicas when needed.
- **Cache**: Redis for feed and profile caching; move to managed later.
- **Object storage + CDN**: Scaleway Object Storage + Edge Services.
- **Queue**: SQS-compatible (Scaleway Messaging); workers consume events.

**Feed strategy**
Start fan-out-on-read with aggressive caching and pagination. As activity grows, add a hybrid path that precomputes timelines for high-activity users and during peak hours.

**Phase 2 (future)**

- Hybrid feed for hot users: precompute recent items into Redis or a feed table.
- Background backfills: rate-limited jobs to refresh caches during peak windows.

## API surface (proposed)

All list endpoints use cursor pagination with `limit` + `cursor`, returning `next_cursor`.

**Auth**

- `POST /auth/token` (admin) create auth token
- `POST /auth/revoke` revoke current token
- `GET /auth/me` return current user identity

**Users & profiles**

- `POST /users` create user (signup)
- `GET /users/:id` fetch user profile
- `PATCH /users/:id` update profile (display name, bio, avatar)
- `GET /users/:id/posts` list posts by user

**Social graph**

- `POST /users/:id/follow` follow user
- `POST /users/:id/unfollow` unfollow user
- `POST /users/:id/block` block user
- `POST /users/:id/unblock` unblock user
- `GET /users/:id/followers` list followers
- `GET /users/:id/following` list following
- `GET /users/:id/relationship` follow/block status between current user and `:id`

**Posts**

- `POST /posts` create post (requires processed media)
- `GET /posts/:id` get post
- `PATCH /posts/:id` update caption
- `DELETE /posts/:id` delete post

**Engagement**

- `POST /posts/:id/like` like post
- `DELETE /posts/:id/like` unlike post
- `GET /posts/:id/likes` list likes
- `POST /posts/:id/comment` add comment
- `GET /posts/:id/comments` list comments
- `DELETE /posts/:id/comments/:comment_id` delete comment

**Feed**

- `GET /feed` home feed (cursor pagination)
- `POST /feed/refresh` optional cache refresh for current user

**Media**

- `POST /media/upload` create upload intent
- `POST /media/upload/:id/complete` finalize upload + enqueue processing
- `GET /media/:id` get media metadata
- `GET /media/upload/:id/status` check processing status
- `DELETE /media/:id` delete media (optional)

**Notifications**

- `GET /notifications` list notifications
- `POST /notifications/:id/read` mark as read

**Moderation/Admin**

- `POST /moderation/users/:id/flag` flag user
- `POST /moderation/posts/:id/takedown` remove post
- `POST /moderation/comments/:id/takedown` remove comment
- `GET /moderation/audit` list moderation actions

**Search/Discovery**

- `GET /search/users?q=` search users by handle/display name
- `GET /search/posts?q=` search posts by caption/hashtags

## Local development (Docker Compose)

### Prerequisites

- Docker and Docker Compose

### Start the stack

```bash
docker compose up --build
```

This brings up:

- Postgres on `localhost:5432`
- Redis on `localhost:6379`
- LocalStack (S3 + SQS) on `localhost:4566`
- API on `http://localhost:8080`

### Seed a test user (one-time)

There is no public "create user" endpoint yet. Insert a user directly:

```bash
docker compose exec db psql -U picshare -d picshare -c \
"INSERT INTO users (id, handle, email, display_name) \
 VALUES ('00000000-0000-0000-0000-000000000001', 'demo', 'demo@example.com', 'Demo User') \
 ON CONFLICT DO NOTHING;"
```

### Seed sample data (users, follows, posts)

```bash
bash docker/seed/seed.sh
```

This inserts a small social graph plus media/post/like/comment records with placeholder S3 keys.

**Optional: upload real images to LocalStack**

- Put files in `docker/seed/images/` (subfolders are preserved)
- For the default seed data, drop files at:
  - `docker/seed/images/alice/coffee.jpg`
  - `docker/seed/images/bob/city.jpg`
  - `docker/seed/images/cora/plate.jpg`
- Run:
  ```bash
  bash docker/seed/upload_media.sh
  ```
  Files are uploaded to `s3://picshare-media/seed/`

### Postman (Docker API)

Use Postman against the Docker Compose API running on `http://localhost:8080`.

**Suggested environment variables**

- `base_url` = `http://localhost:8080`
- `user_id` = `00000000-0000-0000-0000-000000000001`
- `email` = `demo@example.com`
- `password` = `ChangeMe123!`
- `access_token` = (set after login)
- `refresh_token` = (set after login)

**Quick requests**

1. **Health**
   - `GET {{base_url}}/health`
2. **Create user**
   - `POST {{base_url}}/users`
   - Body (JSON):
     ```json
     {
       "handle": "demo",
       "email": "{{email}}",
       "display_name": "Demo User",
       "password": "{{password}}"
     }
     ```
3. **Login**
   - `POST {{base_url}}/auth/login`
   - Body (JSON):
     ```json
     { "email": "{{email}}", "password": "{{password}}" }
     ```
   - Save `access_token` and `refresh_token` from the response.
4. **Get user**
   - `GET {{base_url}}/users/{{user_id}}`
5. **Authenticated requests**
   - Add header: `Authorization: Bearer {{access_token}}`

**Other endpoints to test**

- Public:
  - `GET {{base_url}}/metrics` (501 for now)
- Auth required (add `Authorization: Bearer {{access_token}}`):
  - `POST {{base_url}}/auth/revoke` (Body: `{ "refresh_token": "{{refresh_token}}" }`)
  - `POST {{base_url}}/users/{{user_id}}/follow`
  - `POST {{base_url}}/users/{{user_id}}/unfollow`
  - `POST {{base_url}}/users/{{user_id}}/block`
  - `POST {{base_url}}/users/{{user_id}}/unblock`
  - `GET {{base_url}}/feed?limit=30`
  - `POST {{base_url}}/media/upload`
    - Body (JSON):
      ```json
      { "content_type": "image/jpeg", "bytes": 12345 }
      ```
  - `POST {{base_url}}/media/upload/{{upload_id}}/complete`
  - `POST {{base_url}}/posts`
    - Body (JSON):
      ```json
      { "media_id": "{{media_id}}", "caption": "hello" }
      ```
  - `POST {{base_url}}/posts/{{post_id}}/like`
  - `POST {{base_url}}/posts/{{post_id}}/comment`
    - Body (JSON):
      ```json
      { "body": "Nice shot!" }
      ```
