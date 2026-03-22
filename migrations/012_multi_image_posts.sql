-- Migration: Support multiple images per post via junction table

CREATE TABLE IF NOT EXISTS post_media (
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    media_id UUID NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    position SMALLINT NOT NULL DEFAULT 0,
    PRIMARY KEY (post_id, media_id)
);

CREATE INDEX IF NOT EXISTS idx_post_media_post ON post_media(post_id, position);

-- Migrate legacy single media_id only if the column still exists (re-running migrations
-- on the same DB — e.g. integration tests that replay all SQL files — would otherwise fail).
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'posts'
          AND column_name = 'media_id'
    ) THEN
        INSERT INTO post_media (post_id, media_id, position)
        SELECT id, media_id, 0
        FROM posts
        WHERE media_id IS NOT NULL
        ON CONFLICT DO NOTHING;
    END IF;
END $$;

-- Drop the old media_id column (data has been migrated or was never present)
ALTER TABLE posts DROP COLUMN IF EXISTS media_id;
