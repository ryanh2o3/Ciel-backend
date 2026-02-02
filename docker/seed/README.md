# Seed Data

This directory contains scripts and data to seed the Luminé development environment.

## What gets seeded

- **Users**: alice, bob, cora (demo users)
- **Posts**: 3 posts with images from the seed users
- **Follows**: 
  - shanecp follows alice, bob, and cora
  - alice, bob, and cora follow shanecp back
  - Cross-follows between seed users
- **Likes and Comments**: Sample engagement data
- **Media**: Seed images uploaded to LocalStack S3

## Running the seed script

From the Luminé root directory:

```bash
bash docker/seed/seed.sh
```

This will:
1. Upload all images from `docker/seed/images/` to LocalStack S3
2. Insert users, posts, follows, likes, and comments into the database

## Result

After running the seed script, the shanecp user will have:
- 3 posts in their feed (from alice, bob, and cora)
- 3 followers (alice, bob, cora)
- 3 following (alice, bob, cora)

## Re-seeding

The seed script is idempotent - it uses `ON CONFLICT DO NOTHING` so you can run it multiple times safely.

## Adding more seed data

1. Add images to `docker/seed/images/<username>/`
2. Update `docker/seed/seed.sql` with the new data
3. Run `bash docker/seed/seed.sh`
