-- Migration: Enforce invariants for post_media ordering

-- Ensure position is non-negative (SMALLINT supports negatives by default)
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'post_media_position_non_negative'
    ) THEN
        ALTER TABLE post_media
            ADD CONSTRAINT post_media_position_non_negative
            CHECK (position >= 0);
    END IF;
END $$;

-- Ensure each post has a unique position per media slot
CREATE UNIQUE INDEX IF NOT EXISTS ux_post_media_post_position
    ON post_media (post_id, position);

