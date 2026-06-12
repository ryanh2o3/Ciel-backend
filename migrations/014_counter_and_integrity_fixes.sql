-- 014_counter_and_integrity_fixes.sql
-- 1. Drop the story counter triggers: counters are now maintained exclusively
--    in application code (mark_seen / add_reaction / remove_reaction CTEs).
--    The reaction trigger double-decremented when remove_reaction's CTE
--    deleted the row and decremented in the same statement, and both triggers
--    fired per-row during story-delete cascades for no benefit.
-- 2. One report per (reporter, target): repeat reports could be used to
--    repeatedly drain the target's trust score.
-- 3. Index post_media(media_id) so FK checks on media deletion don't seq-scan.
-- 4. Media referenced by posts/stories can no longer be deleted out from
--    under them (RESTRICT instead of CASCADE, which silently removed images
--    from published posts and deleted stories).

-- 1. Counter triggers
DROP TRIGGER IF EXISTS story_reactions_decrement ON story_reactions;
DROP FUNCTION IF EXISTS trg_decrement_story_reaction_count();

DROP TRIGGER IF EXISTS story_views_decrement ON story_views;
DROP FUNCTION IF EXISTS trg_decrement_story_view_count();

-- 2. Unique (reporter_id, target_id) on user_flags.
-- Dedupe existing rows first (keep the earliest report per pair).
DELETE FROM user_flags uf
USING user_flags older
WHERE uf.reporter_id = older.reporter_id
  AND uf.target_id = older.target_id
  AND (older.created_at < uf.created_at
       OR (older.created_at = uf.created_at AND older.id < uf.id));

CREATE UNIQUE INDEX IF NOT EXISTS ux_user_flags_reporter_target
    ON user_flags(reporter_id, target_id);

-- 3. FK-side index for media deletions
CREATE INDEX IF NOT EXISTS idx_post_media_media ON post_media(media_id);

-- 4. RESTRICT media deletion while referenced by posts or stories
ALTER TABLE post_media
    DROP CONSTRAINT IF EXISTS post_media_media_id_fkey;
ALTER TABLE post_media
    ADD CONSTRAINT post_media_media_id_fkey
    FOREIGN KEY (media_id) REFERENCES media(id) ON DELETE RESTRICT;

ALTER TABLE stories
    DROP CONSTRAINT IF EXISTS stories_media_id_fkey;
ALTER TABLE stories
    ADD CONSTRAINT stories_media_id_fkey
    FOREIGN KEY (media_id) REFERENCES media(id) ON DELETE RESTRICT;
