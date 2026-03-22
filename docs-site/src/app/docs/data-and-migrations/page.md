---
title: Data and migrations
---

## Migrations

SQL migrations live in **`migrations/`** with numeric prefixes (e.g. `001_…sql`, `012_multi_image_posts.sql`). Apply them in order in each environment; Docker Compose applies them automatically in local stacks.

---

## IDs and enums

- **Primary keys** — UUIDs with `DEFAULT uuid_generate_v4()` in SQL. Do not generate primary-key UUIDs in application code.
- **Postgres enums** — Created with `DO $$ BEGIN CREATE TYPE … EXCEPTION WHEN duplicate_object THEN NULL; END $$;` so reruns are safe.

---

## Pagination

**Cursor pagination** uses a tuple `(OffsetDateTime, Uuid)` encoded as a string (e.g. `timestamp/uuid`). Handlers use a **limit + 1** pattern to detect whether a next page exists.

---

## Patterns in SQL

- **CTEs** for atomic insert + counter updates (views, reactions, etc.).
- **Upsert** with `(xmax = 0) AS is_new` in `RETURNING` to distinguish insert vs update where needed.
- **`IF NOT EXISTS`** for tables and indexes.

---

## Visibility and media

Post and story **visibility** is modeled in the database and serialized as text in API responses. Application code maps to/from DB using explicit helpers (`as_db`, `from_db`) rather than deriving sqlx types for enums.

Media metadata and storage paths tie into **object storage** (S3-compatible); processed assets are served via configured public or CDN endpoints.
