-- Migration: Enforce invariants for post_media ordering

-- Ensure position is non-negative (SMALLINT supports negatives by default)
ALTER TABLE post_media
    ADD CONSTRAINT IF NOT EXISTS post_media_position_non_negative
    CHECK (position >= 0);

-- Ensure each post has a unique position per media slot
CREATE UNIQUE INDEX IF NOT EXISTS ux_post_media_post_position
    ON post_media (post_id, position);

