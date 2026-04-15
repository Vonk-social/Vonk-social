-- ============================================================
-- VONK — Phase 2: posts, feed, stories, snaps
-- ============================================================
-- All tables below already exist from 001_initial.sql. This migration
-- only adds the columns/indexes/views that Phase 2 routes need.
-- ============================================================

-- ── Mentions ─────────────────────────────────────────────────
-- Denormalised onto posts so the feed-time visibility check is a single
-- `= ANY(…)` predicate rather than a join into a separate mentions table.
ALTER TABLE posts
    ADD COLUMN mentioned_user_ids BIGINT[] NOT NULL DEFAULT '{}';

CREATE INDEX idx_posts_mentions ON posts USING GIN (mentioned_user_ids);

-- ── Stories ──────────────────────────────────────────────────
-- Stories are posts with post_type='story' + server-set expires_at.
-- A composite index makes "active stories per user" (for the tray) fast.
CREATE INDEX idx_posts_stories_active
    ON posts (user_id, expires_at DESC)
    WHERE post_type = 'story' AND deleted_at IS NULL;

-- Who has seen which story. Used to render the tray "unseen" dot and to
-- let authors see viewer lists later.
CREATE TABLE story_views (
    story_id  BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id   BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    viewed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (story_id, user_id)
);
CREATE INDEX idx_story_views_user ON story_views (user_id, viewed_at DESC);

-- ── Snaps & view policies ────────────────────────────────────
-- The messages table already stores BYTEA ciphertext (Phase 3 will fill it
-- with real MLS-encrypted content). We layer view policy on top so 1-to-1
-- and group snaps can reuse the DM transport without a second table.
ALTER TABLE messages
    ADD COLUMN view_policy TEXT NOT NULL DEFAULT 'persistent'
        CHECK (view_policy IN ('persistent', 'view_once', 'view_24h')),
    ADD COLUMN expires_at TIMESTAMPTZ,
    -- Optional link from a snap message to the media row that holds the bytes
    -- in MinIO. NULL for plain-text DMs. Non-null for snaps.
    ADD COLUMN media_id BIGINT REFERENCES media(id) ON DELETE SET NULL;

CREATE INDEX idx_messages_expires
    ON messages (expires_at)
    WHERE expires_at IS NOT NULL;

CREATE INDEX idx_messages_view_once
    ON messages (conversation_id, created_at DESC)
    WHERE view_policy IN ('view_once', 'view_24h') AND deleted_at IS NULL;

-- Per-recipient consumption tracking for view-once / view-24h messages.
-- This is additive — for regular persistent DMs we do not insert.
CREATE TABLE snap_views (
    message_id  BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    viewed_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (message_id, user_id)
);
CREATE INDEX idx_snap_views_user ON snap_views (user_id, viewed_at DESC);

-- Convenience view: one row per (snap-message, recipient-who-can-see-it).
-- Excludes the sender from recipients, hides already-viewed and expired
-- snaps so the inbox query is a single `SELECT ... WHERE recipient_id = $1
-- AND NOT viewed_by_me`.
CREATE VIEW snap_deliverable AS
SELECT
    m.id,
    m.uuid,
    m.conversation_id,
    m.sender_id,
    cm.user_id            AS recipient_id,
    m.view_policy,
    m.expires_at,
    m.media_id,
    m.created_at,
    EXISTS (
        SELECT 1 FROM snap_views sv
         WHERE sv.message_id = m.id AND sv.user_id = cm.user_id
    ) AS viewed_by_me
FROM messages m
JOIN conversation_members cm
     ON cm.conversation_id = m.conversation_id AND cm.user_id != m.sender_id
WHERE m.deleted_at IS NULL
  AND m.view_policy IN ('view_once', 'view_24h')
  AND (m.expires_at IS NULL OR m.expires_at > now());
