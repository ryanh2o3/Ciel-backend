# Ciel Backend — Security & Architecture Audit

## Table of Contents
- [Application Issues](#application-issues)
  - [Critical](#critical)
  - [High](#high)
  - [Medium](#medium)
  - [Low](#low)
- [Terraform / Infrastructure Issues](#terraform--infrastructure-issues)
  - [Critical](#critical-1)
  - [High](#high-1)
  - [Medium](#medium-1)
  - [Low](#low-1)

---

## Application Issues

### Critical

#### 1. Moderation endpoints have no authorization — any user can take down any post/comment
**Files:** `src/http/handlers.rs:1096-1138`, `src/app/moderation.rs:58-117`

`takedown_post` and `takedown_comment` accept any authenticated user as the `actor_id` and immediately delete the target. There is zero role/permission checking — any logged-in user can delete any other user's post or comment by calling `POST /moderation/posts/:id/takedown`. The service layer just runs `DELETE FROM posts WHERE id = $1` with no ownership or role check.

Similarly, `list_moderation_audit` (`handlers.rs:1140`) exposes the full moderation log to any authenticated user (`_auth: AuthUser` — the `_` prefix means the value is intentionally unused). This leaks internal moderation actions to every user.

#### 2. User emails are exposed in every API response
**File:** `src/domain/user.rs:1-16`

The `User` struct includes `pub email: String` and derives `Serialize`. This struct is returned directly from `get_user` (public, no auth required), `search_users` (public), `list_followers`, `list_following`, and every social endpoint. Any anonymous caller can harvest every user's email address. The email field should be omitted from public-facing serialization or only included in `/auth/me`.

#### 3. Signup has no invite code requirement despite having an invite system
**Files:** `src/http/handlers.rs:256-311`, `src/app/auth.rs:55-88`

The `create_user` handler and `AuthService::signup` accept raw registration with no invite code validation. The `InviteService::consume_invite` method exists but is never called from the signup flow. The entire invite/beta-gate system is dead code — anyone can create an account freely, bypassing the intended invite-only access control.

#### 4. Trust scores are never initialized for new users
**Files:** `src/app/trust.rs:37-49`, `src/app/auth.rs:55-88`

`TrustService::initialize_user()` exists but is never called from the signup flow. New users have no `user_trust_scores` row, which means:
- `get_trust_score` returns `None` for every new user
- Rate limiting middleware falls back to `TrustLevel::New` (line `rate_limit.rs:48`), which works but is accidental
- `get_rate_limits` handler returns 404 "trust score not found" for new users
- `create_invite` fails with "User trust score not found" for new users

---

### High

#### 5. Story reactions endpoint is unauthenticated — anyone can read reactions
**File:** `src/http/handlers.rs:1728-1740`

`list_story_reactions` takes no `AuthUser` extractor, meaning anyone (including unauthenticated users) can enumerate reactions on any story, including friends-only or close-friends-only stories. The story ID is the only required input. This bypasses the visibility model entirely for the reactions data.

#### 6. `revoke_token` endpoint requires no authentication
**File:** `src/http/handlers.rs:175-203`, `src/http/routes.rs:16`

The `/auth/revoke` endpoint has no `AuthUser` extractor. Anyone with a valid refresh token string can revoke it, which is by design for logout flows. However, the endpoint returns different status codes for valid vs. invalid tokens (204 vs. 404), which allows an attacker to probe whether a given token string is valid without authenticating.

#### 7. Race condition in invite creation — quota can be bypassed
**File:** `src/app/invites.rs:39-122`

The invite creation flow does a read-then-write without a transaction:
1. Read `invites_sent` from `user_trust_scores`
2. Compare against `max_invites`
3. Insert into `invite_codes`
4. Update `invites_sent = invites_sent + 1`

Steps 1-4 are not atomic. A user sending concurrent requests can bypass the quota check because all requests read the same `invites_sent` value before any of them increment it. This should use `SELECT ... FOR UPDATE` or a single atomic CTE.

#### 8. `add_reaction` has a non-atomic counter update
**File:** `src/app/stories.rs:179-220`

The upsert of the reaction and the `UPDATE stories SET reaction_count = reaction_count + 1` are in two separate statements (not a transaction). If the app crashes between lines 202 and 209, the reaction is inserted but the counter is never incremented, causing a permanent drift. Compare this to `mark_seen` (line 157) which correctly uses a CTE for atomicity.

#### 9. Search uses ILIKE with user-supplied wildcards — DoS vector
**File:** `src/app/search.rs:26, 81`

```rust
let pattern = format!("%{}%", query);
```

The raw user input is wrapped in `%...%` and passed to `ILIKE`. If a user sends a query like `%` or `_` or `%%`, the pattern becomes `%%%%` which forces a sequential scan of the entire table. There's no escaping of LIKE special characters (`%`, `_`). With a 2-character minimum, an attacker can send `__` to trigger a full table scan. This is a denial-of-service vector, not SQL injection (parameterized queries prevent that), but it can still saturate the database.

#### 10. `login` handler logs the email on failure — potential PII in logs
**File:** `src/http/handlers.rs:116-117`

```rust
tracing::error!(error = ?err, email = %payload.email, "failed to login");
```

The user's email is logged in plaintext on every failed login attempt. If logs are shipped to a third-party service or stored long-term, this creates a PII compliance issue (GDPR, etc.). Login identifiers should be hashed or redacted in logs.

---

### Medium

#### 11. No password length upper bound
**File:** `src/http/handlers.rs:269`

The password validation only enforces a minimum of 8 characters. Argon2 will happily hash a 10MB string, which is a CPU/memory DoS vector. An attacker can send extremely long passwords to consume server resources. Add a maximum length (e.g., 128 characters).

#### 12. `update_profile` returns 401 for non-owner — should be 403
**File:** `src/http/handlers.rs:326-328`

```rust
if auth.user_id != id {
    return Err(AppError::unauthorized("cannot update other users"));
}
```

This returns HTTP 401 (Unauthorized) when it should return 403 (Forbidden). The user is authenticated but not authorized. This is a semantic issue that can confuse clients — 401 implies the auth token is invalid.

#### 13. Feed, media, notifications, moderation, and search routes have no rate limiting
**File:** `src/http/mod.rs:44-58`

Only `auth`, `users`, `posts`, and `stories` route groups have rate limiting middleware. The following are completely unprotected:
- `feed` — could be scraped rapidly
- `media` — upload creation is unprotected
- `notifications` — unbounded polling
- `moderation` — mass-flagging is unprotected (compounding Critical #1)
- `search` — compounding the ILIKE DoS issue (High #9)

#### 14. Presigned URL replacement is hardcoded to `localstack:4566`
**File:** `src/app/media.rs:94, 193`

```rust
upload_url = upload_url.replace("localstack:4566", public_endpoint);
```

The string replacement is hardcoded to the localstack hostname. In production with real S3 (Scaleway Object Storage), the internal hostname won't be `localstack:4566`, so this replacement will silently do nothing. If the production S3 endpoint ever differs from the public endpoint, presigned URLs will contain unreachable internal hostnames.

#### 15. `health` endpoint exposes infrastructure status to the public
**File:** `src/http/handlers.rs:69-75`, `src/http/routes.rs:7-10`

The `/health` endpoint returns per-service status (`db: bool`, `redis: bool`) to unauthenticated callers. This reveals whether specific infrastructure components are down, which is useful reconnaissance for an attacker timing an attack when systems are degraded.

#### 16. No pagination limit on story viewers, reactions, highlights, or feed
**Files:** `src/app/stories.rs:242-291, 412-435, 439-491`

`list_viewers`, `list_reactions`, `get_user_highlights`, `get_stories_feed`, and `get_user_stories` all use `fetch_all` with no `LIMIT` clause. A popular story could have thousands of viewers/reactions, and the entire result set is loaded into memory and serialized in a single response. This is both a memory exhaustion risk and a slow-response DoS vector.

#### 17. `get_user_highlights` is unauthenticated
**File:** `src/http/handlers.rs:1860-1872`

The handler takes no `AuthUser`, so anyone can view any user's highlight collections. While highlights are arguably public content, the endpoint sits in the stories route group alongside authenticated-only endpoints, suggesting this may be an oversight. Compare with `get_user_stories` which requires auth for visibility filtering.

---

### Low

#### 18. `admin_token` in AppState is never used
**File:** `src/main.rs:26`

`AppState` contains `pub admin_token: Option<String>` which is read from the environment but never referenced by any handler or middleware. There is no admin authentication mechanism — the field is dead code.

#### 19. Inconsistent error response for `create_user` failures
**File:** `src/http/handlers.rs:306-307`

When signup fails for a reason other than a unique constraint violation, the error is returned as `AppError::bad_request("failed to create user")` — but the actual cause could be a database connection error (which should be a 500), not a bad request (400).

#### 20. `user_highlights` route conflicts with `users` route group
**File:** `src/http/routes.rs:126-130`

The stories route group registers `/users/:id/stories` and `/users/:id/highlights`, but the users route group also registers paths under `/users/:id/*`. Both route groups are merged in `http/mod.rs`. While Axum handles this correctly because the specific paths differ, it's an organizational smell that could cause conflicts if either group adds overlapping paths.

#### 21. Comment body has no length limit
**File:** `src/http/handlers.rs:782-784`

The only validation is that the body is non-empty after trimming. A user can submit a comment with an arbitrarily large body (limited only by the HTTP body size limit, if one is configured). This could cause storage bloat and slow rendering.

#### 22. `emoji` field on story reactions has no validation beyond non-empty
**File:** `src/http/handlers.rs:1702-1703`

A user can submit any string as an "emoji" — including megabytes of text. This string is stored in the database and returned to all users who view the story's reactions. Add a character length limit (real emoji are 1-7 codepoints).

---

## Terraform / Infrastructure Issues

### Critical

#### 23. Secrets baked into cloud-init plaintext — all secrets in instance metadata
**Files:** `terraform/modules/compute/cloud-init-api.yaml`, `cloud-init-worker.yaml`, `compute/main.tf`

All secrets (DATABASE_URL with password, Redis password, S3 keys, PASETO signing keys, admin token, Scaleway secret key) are interpolated directly into cloud-init templates and written to `/opt/ciel/.env`. Cloud-init user data is typically accessible via the instance metadata service to any process on the machine. The Secrets Manager module stores secrets but they are **never consumed from Secret Manager** — the actual deployment path bypasses it entirely.

#### 24. Scaleway master API key passed to every instance
**Files:** `terraform/environments/prod/main.tf:175`, `terraform/modules/compute/variables.tf`

The `scw_secret_key` (account-level API key) is passed to every API and worker instance via cloud-init for Docker registry login. If any instance is compromised, the attacker gets the full Scaleway account key — not a scoped IAM token.

#### 25. No HTTP-to-HTTPS redirect — credentials sent in cleartext
**File:** `terraform/modules/networking/main.tf:178-186`

The HTTP frontend (port 80) forwards traffic directly to the backend with no redirect to HTTPS. User passwords and PASETO tokens can be transmitted in cleartext if a client connects over HTTP.

#### 26. `ciel` and `prod` environments share the same Terraform state file path
**Files:** `terraform/environments/ciel/backend.tf`, `terraform/environments/prod/backend.tf`

Both environments use the same S3 bucket and key (`ciel-terraform-state` / `prod/terraform.tfstate`). Running `terraform apply` in one environment will corrupt the other's state, potentially destroying production infrastructure.

---

### High

#### 27. Security groups accept traffic from any IP on all ports
**File:** `terraform/modules/networking/main.tf:67-128`

All security groups (API, Worker, Redis) accept traffic from `0.0.0.0/0` on their allowed ports. Redis (port 6379) is particularly dangerous — it's accessible from the public internet, relying solely on password authentication with no IP restriction.

#### 28. SQS credentials have `can_manage = true`
**File:** `terraform/modules/messaging/main.tf:13-25`

The application credentials can create and delete queues. If leaked, an attacker can delete the media processing queue. Only `can_receive` and `can_publish` are needed.

#### 29. Dead letter queue is not connected to the main queue
**File:** `terraform/modules/messaging/main.tf`

The DLQ is created but the main queue has no `redrive_policy` linking them. Failed messages are silently lost rather than routed to the DLQ for investigation.

#### 30. No Terraform state locking
**Files:** `terraform/environments/*/backend.tf`

No `dynamodb_table` or equivalent is configured for state locking. Concurrent `terraform apply` runs can corrupt state.

#### 31. Terraform state stored unencrypted
**Files:** Same backend.tf files

No `encrypt = true` in the S3 backend config. The state file contains every secret in the infrastructure (database passwords, API keys, PASETO keys).

#### 32. Production uses development-tier instances
**File:** `terraform/environments/prod/main.tf:161-163`

`DEV1-M` and `DEV1-S` instances have shared vCPUs and no SLA — unsuitable for production workloads.

#### 33. `container_image_tag` defaults to `latest`
**File:** `terraform/modules/compute/variables.tf:66-67`

Using `latest` in production means non-deterministic deployments, no rollback capability, and invisible drift.

---

### Medium

#### 34. CORS default is `*` (any origin)
**File:** `terraform/modules/storage/variables.tf:29-33`

Default CORS origin allows cross-origin requests from any website. Prod overrides this, but any other environment using the default is wide open.

#### 35. Redis persistence is disabled — all cached data lost on restart
**File:** `terraform/modules/cache/cloud-init-redis.yaml:22-23`

Both RDB and AOF are disabled. Feed caches and rate-limit counters vanish on any Redis restart.

#### 36. No automatic OS security updates after boot
**Files:** All cloud-init files

`package_upgrade: true` only runs at boot. No `unattended-upgrades` is configured, so instances never receive security patches post-provisioning.

#### 37. Worker containers have no health check
**File:** `terraform/modules/compute/cloud-init-worker.yaml`

The API container has a Docker healthcheck, but the worker does not. A zombie worker process won't be detected or restarted.

#### 38. Secrets Manager module is dead code
**File:** `terraform/environments/prod/main.tf:132-146`

Secrets are stored in Scaleway Secret Manager but never consumed from it. The compute module receives secrets directly through Terraform variables, creating a false sense of security.

#### 39. Database user has `ALL` privileges
**File:** `terraform/modules/database/main.tf:50`

The `ciel_app` user has full DDL permissions. Principle of least privilege would restrict it to `SELECT, INSERT, UPDATE, DELETE` and use a separate migration user for schema changes.

#### 40. S3 bucket versioning is disabled
**File:** `terraform/modules/storage/variables.tf:47-51`

Accidental object deletions or overwrites cannot be recovered. The prod environment does not override this default.

#### 41. Observability alerts have no contact emails
**File:** `terraform/modules/observability/variables.tf:41-45`

`alert_contact_emails` defaults to `[]` and prod never provides them. Alerts fire into the void.

#### 42. No WAF or DDoS protection
**File:** `terraform/modules/api_security/main.tf:123-128`

The load balancer is directly exposed to the internet with no WAF, CDN, or infrastructure-level rate limiting.

#### 43. LB-to-backend traffic is unencrypted HTTP
**File:** `terraform/modules/networking/main.tf:161`

Traffic between the load balancer and API instances traverses the private network as plain HTTP. Auth tokens are visible if the private network is compromised.

---

### Low

#### 44. No `prevent_destroy` on database or S3 bucket
**Files:** `terraform/modules/database/main.tf`, `terraform/modules/storage/main.tf`

An accidental `terraform destroy` or resource replacement deletes the production database and all media permanently.

#### 45. Deprecated `template_file` data source
**File:** `terraform/modules/compute/main.tf:16, 46`

Should use the built-in `templatefile()` function instead.

#### 46. Redis binds to `0.0.0.0`
**File:** `terraform/modules/cache/cloud-init-redis.yaml:12`

Should bind only to the private network interface. Currently listens on all interfaces, including any public IP.

#### 47. Managed Redis cluster size defaults to 1
**File:** `terraform/modules/cache/variables.tf:62-66`

No redundancy — a single node failure takes down the entire cache layer.

#### 48. Docker Compose version '3.8' is deprecated
**Files:** Both cloud-init YAML files

The `version` key is obsolete in Docker Compose V2.

#### 49. Grafana user defaults to `editor` role
**File:** `terraform/modules/observability/variables.tf:32`

Should default to `viewer` for read-only dashboards following least-privilege.

---

## Summary

| Severity | App | Terraform | Total |
|----------|-----|-----------|-------|
| Critical | 4   | 4         | **8** |
| High     | 6   | 7         | **13**|
| Medium   | 7   | 10        | **17**|
| Low      | 5   | 6         | **11**|
| **Total**| **22** | **27** | **49**|

## Top Priorities

1. **Add role-based authorization to moderation endpoints** (Critical #1) — Any user can delete anyone's content right now.
2. **Remove `email` from public User serialization** (Critical #2) — Every user's email is exposed to anonymous callers.
3. **Wire up invite validation in the signup flow** (Critical #3) — The invite gate is completely bypassed.
4. **Initialize trust scores on signup** (Critical #4) — Multiple features break for new users.
5. **Stop baking secrets into cloud-init** (Critical #23) — Pull from Secret Manager at runtime instead.
6. **Fix the shared Terraform state path** (Critical #26) — `ciel` and `prod` will corrupt each other's state.
7. **Add HTTPS redirect** (Critical #25) — Credentials can currently be sent in cleartext.
8. **Restrict security groups** (High #27) — Redis is internet-accessible.
