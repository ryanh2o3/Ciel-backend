# Ciel Backend Testing Strategy

This document outlines a comprehensive testing strategy for the Ciel backend, prioritized by **impact**, **safety**, and **stability**. The focus is on **service-level integration tests** that verify the behavior routes *should* have—not just what the code currently does.

---

## Table of Contents

1. [Testing Philosophy](#testing-philosophy)
2. [Priority 1: Critical Security & Authentication](#priority-1-critical-security--authentication)
3. [Priority 2: Core User Operations](#priority-2-core-user-operations)
4. [Priority 3: Social Graph & Blocking](#priority-3-social-graph--blocking)
5. [Priority 4: Content Creation & Media](#priority-4-content-creation--media)
6. [Priority 5: Engagement (Likes, Comments)](#priority-5-engagement-likes-comments)
7. [Priority 6: Stories & Ephemeral Content](#priority-6-stories--ephemeral-content)
8. [Priority 7: Feed & Search](#priority-7-feed--search)
9. [Priority 8: Safety & Anti-Abuse](#priority-8-safety--anti-abuse)
10. [Priority 9: Moderation System](#priority-9-moderation-system)
11. [Edge Cases & Boundary Testing](#edge-cases--boundary-testing)
12. [Test Infrastructure Recommendations](#test-infrastructure-recommendations)

---

## Testing Philosophy

> [!IMPORTANT]
> **Test what routes SHOULD do, not what code currently does.**
> Each test should verify expected behavior based on API contracts and business requirements.

### Test Priorities
- **🔴 P0 (Critical)**: Security, authentication, authorization failures = data breach risk
- **🟠 P1 (High)**: Core operations that can corrupt data or break user experience  
- **🟡 P2 (Medium)**: Important features that affect user experience
- **🟢 P3 (Low)**: Nice-to-have coverage

---

## Priority 1: Critical Security & Authentication

### 🔴 P0-001: Login Security

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `login_valid_credentials` | POST `/auth/login` with correct email/password | Returns 200 with `access_token`, `refresh_token`, expiry timestamps |
| `login_invalid_password` | POST `/auth/login` with wrong password | Returns 401 "invalid credentials" — NOT 500 |
| `login_nonexistent_user` | POST `/auth/login` with email that doesn't exist | Returns 401 "invalid credentials" (same as wrong password—no user enumeration) |
| `login_empty_email` | POST `/auth/login` with empty email | Returns 400 "email and password are required" |
| `login_empty_password` | POST `/auth/login` with empty password | Returns 400 |
| `login_password_too_long` | POST `/auth/login` with 150-char password | Returns 400 "password must be at most 128 characters" |
| `login_sql_injection_email` | POST `/auth/login` with `'; DROP TABLE users;--` as email | Returns 401 (no crash, no SQL error exposed) |
| `login_sql_injection_password` | Same for password field | Returns 401 |

### 🔴 P0-002: Token Lifecycle

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `refresh_valid_token` | POST `/auth/refresh` with valid refresh token | Returns new access/refresh tokens |
| `refresh_expired_token` | POST `/auth/refresh` with expired refresh token | Returns 401 "invalid refresh token" |
| `refresh_revoked_token` | Revoke token, then try to refresh | Returns 401 |
| `refresh_malformed_token` | POST `/auth/refresh` with random string | Returns 401 |
| `refresh_empty_token` | POST `/auth/refresh` with empty string | Returns 400 "refresh_token is required" |
| `revoke_own_token` | POST `/auth/revoke` with valid token | Returns 204, token unusable |
| `revoke_already_revoked` | Revoke same token twice | Returns 204 (idempotent) |
| `access_token_expiry` | Use access token after TTL | Returns 401 |

### 🔴 P0-003: Protected Route Authorization

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `get_current_user_no_token` | GET `/auth/me` without Authorization header | Returns 401 |
| `get_current_user_invalid_token` | GET `/auth/me` with garbage token | Returns 401 |
| `get_current_user_expired_token` | GET `/auth/me` with expired token | Returns 401 |
| `create_post_no_auth` | POST `/posts` without token | Returns 401 |
| `admin_endpoint_no_admin_token` | POST `/moderation/posts/:id/takedown` without `X-Admin-Token` | Returns 401/403 |
| `admin_endpoint_wrong_admin_token` | Same with wrong admin token | Returns 401/403 |

---

## Priority 2: Core User Operations

### 🟠 P1-001: User Registration

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `signup_valid_data` | POST `/users` with valid handle/email/password/invite | Returns user object with ID |
| `signup_duplicate_handle` | Try to register with existing handle | Returns 409 "Handle already taken" |
| `signup_duplicate_email` | Try to register with existing email | Returns 409 "Email already taken" |
| `signup_invalid_invite_code` | Register with non-existent invite | Returns 400 (invite error) |
| `signup_expired_invite_code` | Register with expired invite | Returns 400 |
| `signup_already_used_invite` | Register with used invite | Returns 400 |
| `signup_revoked_invite` | Register with revoked invite | Returns 400 |
| `signup_handle_too_short` | Handle with 2 chars | Returns 400 "handle must be at least 3 characters" |
| `signup_handle_too_long` | Handle with 31 chars | Returns 400 "handle must be at most 30 characters" |
| `signup_handle_special_chars` | Handle with `@#$%` | Returns 400 "handle can only contain letters, numbers, and underscores" |
| `signup_password_too_short` | Password with 7 chars | Returns 400 "password must be at least 8 characters" |
| `signup_password_too_long` | Password with 129 chars | Returns 400 |
| `signup_bio_too_long` | Bio with 501 chars | Returns 400 "bio must be at most 500 characters" |
| `signup_display_name_empty` | Empty display name | Returns 400 "display_name cannot be empty" |
| `signup_display_name_too_long` | Display name 51 chars | Returns 400 |

### 🟠 P1-002: Profile Management

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `update_own_profile` | PATCH `/users/:id` own profile with new display_name | Returns updated user |
| `update_other_user_profile` | PATCH `/users/:id` with different user's ID | Returns 403 "cannot update other users" |
| `update_profile_empty_display_name` | Update with empty display_name | Returns 400 |
| `update_profile_bio_too_long` | Update with 501-char bio | Returns 400 |
| `get_user_by_id` | GET `/users/:id` | Returns public user object |
| `get_nonexistent_user` | GET `/users/:invalid_uuid` | Returns 404 "user not found" |
| `get_user_malformed_uuid` | GET `/users/not-a-uuid` | Returns 400 (or 404) |

### 🟠 P1-003: Account Deletion (GDPR/CCPA)

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `delete_own_account` | DELETE `/account` | Returns 204, user cannot login afterward |
| `delete_account_cascades_posts` | After deletion, user's posts are gone | Expect posts deleted or orphaned appropriately |
| `delete_account_cascades_follows` | Follower counts update | Followee loses a follower |
| `delete_account_invalidates_tokens` | After deletion, old tokens fail | Returns 401 |

---

## Priority 3: Social Graph & Blocking

### 🟠 P1-004: Follow System

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `follow_user` | POST `/users/:id/follow` | Returns `{ followed: true }` |
| `follow_already_following` | Follow same user twice | Returns `{ followed: false }` (idempotent) |
| `follow_self` | POST `/users/:own_id/follow` | Returns 400 "cannot follow yourself" |
| `follow_nonexistent_user` | Follow user that doesn't exist | Returns 404 or fails gracefully |
| `unfollow_user` | POST `/users/:id/unfollow` | Returns `{ unfollowed: true }` |
| `unfollow_not_following` | Unfollow user not in following list | Returns `{ unfollowed: false }` |
| `unfollow_self` | POST `/users/:own_id/unfollow` | Returns 400 |
| `list_followers_pagination` | GET `/users/:id/followers?limit=5&cursor=...` | Returns paginated list with `next_cursor` |
| `list_following_pagination` | GET `/users/:id/following` | Returns paginated list |
| `follow_updates_counts` | After follow, follower_count increments | Verify via GET user |

### 🟠 P1-005: Block System

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `block_user` | POST `/users/:id/block` | Returns `{ blocked: true }` |
| `block_removes_follow` | If A follows B, then A blocks B → follow removed | Verify follow is gone |
| `blocked_user_cannot_follow` | If A blocked B, B tries to follow A | Should fail or be prevented |
| `block_self` | POST `/users/:own_id/block` | Returns 400 "cannot block yourself" |
| `unblock_user` | POST `/users/:id/unblock` | Returns `{ unblocked: true }` |
| `relationship_status` | GET `/users/:id/relationship` | Returns `{ is_following, is_followed_by, is_blocking, is_blocked_by }` |
| `relationship_status_self` | GET `/users/:own_id/relationship` | Returns all false flags |

### 🟠 P1-006: Block Enforcement

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `blocked_user_hidden_in_feed` | If A blocks B, B's posts don't appear in A's feed | Verify via GET `/feed` |
| `blocked_user_hidden_in_search` | B shouldn't appear when A searches | Verify via `/search/users` |
| `blocked_user_cannot_see_posts` | B cannot see A's posts directly | GET `/posts/:id` returns 404 or filtered |
| `blocked_user_cannot_see_stories` | B cannot view A's stories | Verify story visibility |

---

## Priority 4: Content Creation & Media

### 🟠 P1-007: Media Upload Flow

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `create_upload_valid` | POST `/media/upload` with content_type, bytes | Returns `upload_id`, presigned URL |
| `create_upload_zero_bytes` | bytes = 0 | Returns 400 "bytes must be greater than 0" |
| `create_upload_negative_bytes` | bytes = -1 | Returns 400 |
| `create_upload_exceeds_max` | bytes > `upload_max_bytes` config | Returns 400 "upload exceeds max size" |
| `create_upload_invalid_content_type` | content_type = "text/html" | Should fail or be restricted |
| `complete_upload` | POST `/media/upload/:id/complete` | Returns 202 (processing queued) |
| `complete_upload_wrong_user` | Complete another user's upload | Returns 404 or 403 |
| `complete_upload_twice` | Complete same upload twice | Should be idempotent or error |
| `get_upload_status` | GET `/media/upload/:id/status` | Returns processing state |
| `get_media` | GET `/media/:id` | Returns media object with URLs |
| `get_media_wrong_user` | Unauthenticated or different user | Depends on visibility rules |
| `delete_media` | DELETE `/media/:id` | Returns 204, media gone |
| `delete_media_wrong_user` | Delete another user's media | Returns 404 or 403 |

### 🟠 P1-008: Post Creation

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `create_post_valid` | POST `/posts` with media_id, caption | Returns post object |
| `create_post_no_media` | POST `/posts` without media_id | Returns 400 or fails |
| `create_post_invalid_media` | POST with non-existent media_id | Returns error |
| `create_post_other_users_media` | POST with media belonging to another user | Should fail |
| `create_post_caption_too_long` | Caption with 2201 chars | Returns 400 "caption must be at most 2200 characters" |
| `get_post` | GET `/posts/:id` | Returns post with media URLs |
| `get_nonexistent_post` | GET `/posts/:invalid_id` | Returns 404 |
| `update_post_caption` | PATCH `/posts/:id` with new caption | Returns updated post |
| `update_post_wrong_user` | PATCH another user's post | Returns 404 (ownership enforced) |
| `delete_post` | DELETE `/posts/:id` | Returns 204 |
| `delete_post_wrong_user` | DELETE another user's post | Returns 404 |
| `list_user_posts` | GET `/users/:id/posts` | Returns paginated posts |

---

## Priority 5: Engagement (Likes, Comments)

### 🟡 P2-001: Likes

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `like_post` | POST `/posts/:id/like` | Returns `{ created: true }` |
| `like_post_twice` | Like same post again | Returns `{ created: false }` (idempotent) |
| `unlike_post` | DELETE `/posts/:id/like` | Returns 204 |
| `unlike_not_liked` | Unlike post not in likes | Returns 404 "like not found" |
| `like_nonexistent_post` | Like post that doesn't exist | Returns error |
| `list_post_likes` | GET `/posts/:id/likes` | Returns paginated list of likes |
| `like_updates_count` | After like, `like_count` on post increments | Verify via GET post |

### 🟡 P2-002: Comments

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `comment_on_post` | POST `/posts/:id/comment` with body | Returns comment object |
| `comment_empty_body` | POST with empty body | Returns 400 "comment body cannot be empty" |
| `comment_body_too_long` | Body with 1001 chars | Returns 400 "comment body exceeds 1000 characters" |
| `comment_on_nonexistent_post` | Comment on invalid post | Returns error |
| `list_post_comments` | GET `/posts/:id/comments` | Returns paginated comments |
| `delete_own_comment` | DELETE `/posts/:post_id/comments/:comment_id` | Returns 204 |
| `delete_comment_wrong_user` | Delete another user's comment | Returns 404 |
| `delete_comment_post_owner` | Post owner deletes any comment | **Verify expected behavior** |

---

## Priority 6: Stories & Ephemeral Content

### 🟡 P2-003: Story Creation & Visibility

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `create_story` | POST `/stories` with media_id, visibility | Returns story object |
| `create_story_invalid_media` | Story with non-existent media | Returns error |
| `create_story_other_users_media` | Story with another user's media | Returns 403 "media does not belong to you" |
| `get_story` | GET `/stories/:id` | Returns story if visible |
| `get_expired_story` | GET story after 24h TTL | Returns 404 (or owner can still see) |
| `delete_story` | DELETE `/stories/:id` | Returns 204 |
| `delete_story_wrong_user` | Delete another user's story | Returns 404 |
| `story_visibility_private` | Private story not visible to non-followers | Verify via GET |
| `story_visibility_followers` | Followers-only story visible to followers | Verify |

### 🟡 P2-004: Story Interactions

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `mark_story_seen` | POST `/stories/:id/seen` | Returns 204 |
| `mark_seen_twice` | Mark same story seen twice | Returns 204 (idempotent, no duplicate) |
| `add_story_reaction` | POST `/stories/:id/reactions` with emoji | Returns reaction object |
| `add_reaction_replaces` | Add different emoji → replaces previous | Single reaction per user |
| `add_reaction_empty_emoji` | Empty emoji string | Returns 400 "emoji cannot be empty" |
| `add_reaction_emoji_too_long` | Emoji 8+ chars | Returns 400 |
| `remove_story_reaction` | DELETE `/stories/:id/reactions` | Returns 204 |
| `list_story_reactions` | GET `/stories/:id/reactions` | Returns paginated reactions |
| `get_story_viewers` | GET `/stories/:id/viewers` | Returns viewers (owner only) |
| `get_story_viewers_wrong_user` | Non-owner tries to view viewers | Returns 403 |
| `get_story_metrics` | GET `/stories/:id/metrics` | Returns view/reaction counts (owner only) |

### 🟡 P2-005: Highlights

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `add_story_to_highlight` | POST `/stories/:id/highlights` | Creates/updates highlight |
| `add_to_highlight_empty_name` | Empty highlight name | Returns 400 |
| `add_expired_story_to_highlight` | Add story after 24h | Should still work (persisting) |
| `get_user_highlights` | GET `/users/:id/highlights` | Returns paginated highlights |

---

## Priority 7: Feed & Search

### 🟡 P2-006: Home Feed

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `get_home_feed` | GET `/feed` | Returns posts from followed users |
| `home_feed_respects_blocks` | Blocked users' posts excluded | Verify |
| `home_feed_pagination` | GET `/feed?limit=5&cursor=...` | Returns paginated, chronological |
| `home_feed_empty` | New user with no follows | Returns empty items |
| `refresh_feed` | POST `/feed/refresh` | Returns 204, cache invalidated |
| `stories_feed` | GET `/feed/stories` | Returns active stories from followed |
| `stories_feed_excludes_expired` | Stories > 24h old not shown | Verify |

### 🟡 P2-007: Search

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `search_users` | GET `/search/users?q=john` | Returns matching users |
| `search_users_short_query` | GET `/search/users?q=a` | Returns 400 "q must be at least 2 characters" |
| `search_users_respects_blocks` | Blocked users not in results | Verify |
| `search_posts` | GET `/search/posts?q=sunset` | Returns matching posts |
| `search_posts_pagination` | Use cursor for next page | Returns paginated |

---

## Priority 8: Safety & Anti-Abuse

### 🟠 P1-009: Invite System

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `create_invite` | POST `/invites` | Returns invite code |
| `create_invite_max_limit` | Exceed max invites per user | Returns 403 "Maximum invite limit" |
| `create_invite_invalid_days` | days_valid = 0 or 31 | Returns 400 "days_valid must be between 1 and 30" |
| `list_invites` | GET `/invites` | Returns user's invite codes |
| `get_invite_stats` | GET `/invites/stats` | Returns total/used/remaining |
| `revoke_invite` | POST `/invites/:code/revoke` | Returns 204 |
| `revoke_used_invite` | Revoke already-used invite | Returns 404 |
| `revoke_other_users_invite` | Revoke invite you don't own | Returns 404 |

### 🟠 P1-010: Trust System

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `get_trust_score` | GET `/account/trust-score` | Returns trust level, points, status |
| `get_rate_limits` | GET `/account/rate-limits` | Returns current limits and remaining quotas |
| `trust_level_affects_limits` | Higher trust = higher limits | Verify quotas match trust level |
| `rate_limit_exceeded` | Exceed posts_per_hour | Returns 429 or blocked |

### 🟠 P1-011: Device Fingerprinting

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `register_device` | POST `/account/device/register` with fingerprint | Returns 204 |
| `register_device_blocked` | Blocked fingerprint tries to register | Returns 403 "This device has been blocked" |
| `list_user_devices` | GET `/account/devices` | Returns device list with risk scores |
| `high_risk_device_flagged` | Device with risk_score > 80 | Logged as warning |

---

## Priority 9: Moderation System

### 🟡 P2-008: User Flagging

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `flag_user` | POST `/moderation/users/:id/flag` | Returns flag object |
| `flag_user_with_reason` | Include reason in body | Reason stored |
| `flag_self` | Flag your own account | Should fail or be allowed (policy decision) |

### 🟡 P2-009: Admin Actions

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `takedown_post` | POST `/moderation/posts/:id/takedown` (admin) | Returns 204, post hidden |
| `takedown_post_not_admin` | Same without admin token | Returns 401/403 |
| `takedown_comment` | POST `/moderation/comments/:id/takedown` (admin) | Returns 204 |
| `list_moderation_audit` | GET `/moderation/audit` (admin) | Returns audit log |
| `audit_not_admin` | Same without admin token | Returns 401/403 |

---

## Edge Cases & Boundary Testing

### 🟢 P3-001: Pagination Edge Cases

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `pagination_limit_zero` | `?limit=0` | Returns 400 |
| `pagination_limit_negative` | `?limit=-1` | Returns 400 |
| `pagination_limit_too_high` | `?limit=201` | Returns 400 "limit must be between 1 and 200" |
| `pagination_invalid_cursor` | Malformed cursor string | Returns 400 "invalid cursor" |
| `pagination_cursor_with_invalid_uuid` | Cursor with bad UUID | Returns 400 |
| `pagination_cursor_with_invalid_timestamp` | Cursor with bad RFC3339 | Returns 400 |

### 🟢 P3-002: UUID Handling

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `invalid_uuid_in_path` | GET `/users/not-a-uuid` | Returns 400 |
| `null_uuid` | GET `/users/00000000-0000-0000-0000-000000000000` | Returns 404 |
| `uuid_case_insensitive` | UUID with uppercase | Should work |

### 🟢 P3-003: Concurrent Operations

| Test Case | What to Test | Expected Result |
|-----------|--------------|-----------------|
| `concurrent_likes` | 10 simultaneous like requests | Only one like created |
| `concurrent_follows` | 10 simultaneous follow requests | Only one follow created |
| `concurrent_comment_delete` | Delete same comment twice simultaneously | One succeeds, one 404 |

---

## Test Infrastructure Recommendations

### Database Setup
```rust
// Use test database with transactions that rollback
#[sqlx::test]
async fn test_create_user(pool: PgPool) {
    // Each test gets isolated transaction
}
```

### Test Fixtures

Create fixture factory functions:
- `create_test_user(pool)` → Returns user + auth tokens
- `create_test_post(pool, user_id)` → Returns post
- `create_test_invite(pool, user_id)` → Returns invite code

### API Client Helper
```rust
struct TestClient {
    app: Router,
    auth_token: Option<String>,
}

impl TestClient {
    async fn get(&self, path: &str) -> Response;
    async fn post_json<T: Serialize>(&self, path: &str, body: T) -> Response;
    fn with_auth(mut self, token: &str) -> Self;
}
```

### Running Tests

```bash
# Unit tests
cargo test --lib

# Integration tests (requires database)
DATABASE_URL=postgres://... cargo test --test '*'

# Specific test module
cargo test auth::tests

# With logging
RUST_LOG=debug cargo test -- --nocapture
```

---

## Implementation Order

Based on impact and dependencies, implement tests in this order:

### Phase 1: Security Foundation (Week 1)
1. All P0 authentication tests
2. All P0 authorization tests
3. P1 user registration tests

### Phase 2: Core Operations (Week 2)
1. P1 profile management
2. P1 follow/block system
3. P1 media upload flow

### Phase 3: Content & Engagement (Week 3)
1. P1 post CRUD
2. P2 likes & comments
3. P2 stories

### Phase 4: Anti-Abuse & Moderation (Week 4)
1. P1 invite system
2. P1 trust/rate limiting
3. P2 moderation endpoints

### Phase 5: Edge Cases & Polish (Week 5)
1. P3 pagination edge cases
2. P3 concurrent operation tests
3. Load testing basics

---

## Success Metrics

- **Coverage Target**: 80%+ line coverage on handlers
- **All P0 tests passing**: Required for any deploy
- **All P1 tests passing**: Required for production release
- **P2/P3 tests**: Should pass, failures are warnings

> [!TIP]
> Start with the P0 security tests. A single authentication bypass is worse than 100 missing feature tests.
