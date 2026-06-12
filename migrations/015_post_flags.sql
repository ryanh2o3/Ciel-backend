-- User-facing post reports (UGC safety). One report per (reporter, post).

CREATE TABLE IF NOT EXISTS post_flags (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    reporter_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_post_flags_reporter_post
    ON post_flags(reporter_id, post_id);

CREATE INDEX IF NOT EXISTS idx_post_flags_post
    ON post_flags(post_id, created_at DESC);
