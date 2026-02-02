## Ciel API Integration Guide

This document describes how a client app connects to the Ciel backend, how to use auth, and how to call every endpoint. It also includes data access and pagination best practices for scalable clients.

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

### Rate Limiting & Safety Features

Ciel implements comprehensive safety and anti-abuse measures:

#### Rate Limiting
- All authenticated endpoints are rate-limited based on user trust level
- Rate limits use sliding window algorithm with Redis backend
- When rate limited, API returns `429 Too Many Requests` with JSON body:
```json
{ "error": "Rate limit exceeded for action: post" }
```

#### Trust Levels
Users progress through 4 trust levels with increasing rate limits:

| Trust Level | Posts/Hour | Posts/Day | Follows/Day | Likes/Hour |
|-------------|------------|-----------|-------------|------------|
| New         | 1          | 5         | 20          | 30         |
| Basic       | 5          | 20        | 100         | 100        |
| Trusted     | 20         | 100       | 500         | 500        |
| Verified    | 50         | 200       | 1000        | 1000       |

**Trust Progression:**
- New → Basic: 7 days + 5 posts + 20 trust points
- Basic → Trusted: 90 days + 50 posts + 200 trust points
- Trusted → Verified: Manual promotion

#### Device Fingerprinting
- Multi-account detection using FingerprintJS
- Required for signup and login
- Helps prevent bot networks and abuse

#### Invite-Only Signup
- Users need valid invite code to signup
- Invite quotas based on trust level:
  - New: 3 invites
  - Basic: 10 invites
  - Trusted: 50 invites
  - Verified: 200 invites

### Best Practices for Safety Integration

1. **Handle Rate Limits Gracefully:**
   - Catch `429` errors and show user-friendly messages
   - Implement exponential backoff for retries
   - Display remaining quota in UI when approaching limits

2. **Device Fingerprinting:**
   - Integrate FingerprintJS library
   - Send fingerprint on signup/login
   - Store fingerprint securely

3. **Invite System:**
   - Request invite codes from users during signup
   - Show invite management UI for creating/revoking invites
   - Display invite statistics

4. **Trust System:**
   - Show trust level and progress in user profile
   - Display rate limits and remaining quotas
   - Provide feedback on actions that earn trust points

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
  "display_name": "string",
  "bio": "string|null",
  "avatar_key": "string|null",
  "created_at": "RFC3339"
}
```

**Note:** Email field has been removed from public User responses for privacy reasons. Email is only available through `/auth/me` endpoint for the authenticated user.

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

### Safety & Trust Endpoints (auth required)

**GET `/account/trust-score`**
- Response:
```json
{
  "user_id": "uuid",
  "trust_level": 1,
  "trust_level_name": "Basic",
  "trust_points": 45,
  "account_age_days": 14,
  "posts_count": 8,
  "followers_count": 12,
  "strikes": 0,
  "is_banned": false
}
```

**GET `/account/rate-limits`**
- Response:
```json
{
  "trust_level": "Basic",
  "posts_per_hour": 5,
  "posts_per_day": 20,
  "follows_per_hour": 20,
  "follows_per_day": 100,
  "likes_per_hour": 100,
  "comments_per_hour": 30,
  "remaining": {
    "posts": 4,
    "follows": 18,
    "likes": 95,
    "comments": 28
  }
}
```

**POST `/account/device/register`**
- Body:
```json
{
  "fingerprint": "sha256-hash-from-fingerprintjs"
}
```
- Response: `204 No Content`

**GET `/account/devices`**
- Response:
```json
{
  "devices": [
    {
      "fingerprint": "sha256-hash",
      "risk_score": 5,
      "first_seen_at": "RFC3339",
      "last_seen_at": "RFC3339",
      "account_count": 2
    }
  ]
}
```

### Invite System Endpoints (auth required)

**GET `/invites`**
- Response:
```json
{
  "invites": [
    {
      "code": "A1B2C3D4E5F6",
      "created_by": "uuid",
      "used_by": null,
      "created_at": "RFC3339",
      "used_at": null,
      "expires_at": "RFC3339",
      "is_valid": true,
      "invite_type": "standard",
      "use_count": 0,
      "max_uses": 1
    }
  ],
  "quota_used": 2,
  "quota_max": 10
}
```

**POST `/invites`**
- Body:
```json
{
  "days_valid": 7
}
```
- Response:
```json
{
  "code": "A1B2C3D4E5F6",
  "created_by": "uuid",
  "used_by": null,
  "created_at": "RFC3339",
  "used_at": null,
  "expires_at": "RFC3339",
  "is_valid": true,
  "invite_type": "standard",
  "use_count": 0,
  "max_uses": 1
}
```

**GET `/invites/stats`**
- Response:
```json
{
  "total_created": 5,
  "total_used": 3,
  "total_expired": 1,
  "total_revoked": 1,
  "successful_invites": 3,
  "trust_points_earned": 30
}
```

**POST `/invites/:code/revoke`**
- Response: `204 No Content`

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
  "password": "ChangeMe123!",
  "invite_code": "A1B2C3D4E5F6"
}
```
- Response: `User`

**Note:** Invite code is required for signup. Users must obtain an invite code from existing users.

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

### Safety Integration Best Practices

#### Rate Limit Handling
```swift
// Example: Handle rate limit errors gracefully
func performAction() async {
    do {
        let result = try await apiClient.send(request)
        // Success
    } catch APIError.serverError(let message, let statusCode) {
        if statusCode == 429 {
            // Show user-friendly rate limit message
            showAlert(title: "Rate Limit", message: "You've reached your limit for this action. Please try again later.")
            
            // Optional: Fetch remaining quotas
            let rateLimits = try await apiClient.send(RateLimitsRequest())
            updateUIWithRemainingQuotas(rateLimits.remaining)
        }
    }
}
```

#### Device Fingerprinting Integration
```swift
// Example: Register device fingerprint on login
import FingerprintJS

func login(email: String, password: String) async throws -> AuthResponse {
    // Get device fingerprint
    let fingerprint = try await FingerprintJS.getFingerprint()
    
    // Send to backend
    let request = DeviceRegisterRequest(fingerprint: fingerprint)
    try await apiClient.sendNoContent(request)
    
    // Proceed with login
    let authRequest = LoginRequest(email: email, password: password)
    return try await apiClient.send(authRequest)
}
```

#### Invite System Integration
```swift
// Example: Handle invite code during signup
func signup(withInviteCode inviteCode: String) async throws -> User {
    let request = SignupRequest(
        handle: handle,
        email: email,
        displayName: displayName,
        password: password,
        inviteCode: inviteCode
    )
    return try await apiClient.send(request)
}
```

#### Trust System UI Integration
```swift
// Example: Show trust level and progress
func loadTrustInfo() async {
    let trustScore = try await apiClient.send(TrustScoreRequest())
    let rateLimits = try await apiClient.send(RateLimitsRequest())
    
    updateUI(
        trustLevel: trustScore.trustLevelName,
        trustPoints: trustScore.trustPoints,
        nextLevelPoints: trustScore.nextLevelPoints,
        remainingPosts: rateLimits.remaining.posts
    )
}
```

### Error Handling Enhancements

Add new error cases to handle safety features:

```swift
extension APIError {
    case rateLimited(action: String, retryAfter: TimeInterval?)
    case inviteRequired
    case deviceBlocked
    case trustLevelInsufficient
}
```

### Monitoring & Analytics

Track safety-related metrics:
- Rate limit violations (by action type)
- Trust level progression events
- Invite code usage and success rates
- Device fingerprint registrations

**Do NOT log:**
- User email addresses
- Device fingerprints
- Invite codes
- Rate limit quotas

