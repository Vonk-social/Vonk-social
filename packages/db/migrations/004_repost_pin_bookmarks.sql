-- ============================================================
-- VONK — Phase 2.5: reposts, pins (bookmarks table already exists)
-- ============================================================
-- Adds:
--   - posts.repost_of_id: link a repost post to the original
--   - posts.pinned_at: one of up to 3 posts pinned on an author's profile
-- Everything else that's needed (bookmarks table, reply_count, like_count)
-- already exists in 001_initial.sql.
-- ============================================================

ALTER TABLE posts
    ADD COLUMN repost_of_id BIGINT REFERENCES posts(id) ON DELETE SET NULL,
    ADD COLUMN pinned_at TIMESTAMPTZ;

-- Indexes for the two new access patterns we ship now.

-- Find a user's reposts of a given post, and count reposts per post.
CREATE INDEX idx_posts_repost_of ON posts (repost_of_id)
    WHERE repost_of_id IS NOT NULL;

-- Load pinned posts for a profile page fast, without scanning all of the
-- author's posts. Partial index because the overwhelming majority of rows
-- will have pinned_at IS NULL.
CREATE INDEX idx_posts_pinned ON posts (user_id, pinned_at DESC)
    WHERE pinned_at IS NOT NULL AND deleted_at IS NULL;
