-- ============================================================
-- VONK — Database Schema v1
-- PostgreSQL 16+
-- 
-- Designed to start simple and scale to Citus when needed.
-- All user-scoped tables include user_id for future sharding.
-- ============================================================

-- Extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "citext";       -- case-insensitive text for emails/usernames

-- ============================================================
-- USERS & AUTH
-- ============================================================

CREATE TABLE users (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,
    username        CITEXT NOT NULL UNIQUE,
    display_name    TEXT NOT NULL,
    email           CITEXT UNIQUE,             -- NULL if SSO-only without email disclosure
    email_verified  BOOLEAN DEFAULT false,
    bio             TEXT DEFAULT '',
    avatar_url      TEXT,
    banner_url      TEXT,
    location_city   TEXT,                      -- Optional, city-level only (privacy)
    location_country TEXT,
    locale          TEXT DEFAULT 'nl',
    is_private      BOOLEAN DEFAULT false,     -- Private accounts require follow approval
    is_suspended    BOOLEAN DEFAULT false,
    suspended_reason TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ,               -- Soft delete (hard delete after GDPR request)
    
    CONSTRAINT username_format CHECK (username ~ '^[a-z0-9_]{3,30}$')
);

CREATE INDEX idx_users_username ON users (username);
CREATE INDEX idx_users_email ON users (email) WHERE email IS NOT NULL;
CREATE INDEX idx_users_created ON users (created_at);

-- SSO / OAuth connections (one user can have multiple providers)
CREATE TABLE user_auth_providers (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider        TEXT NOT NULL,             -- 'google', 'apple', 'github', 'itsme'
    provider_uid    TEXT NOT NULL,             -- Provider's unique user ID
    provider_email  TEXT,
    provider_name   TEXT,
    access_token    TEXT,                      -- Encrypted at rest
    refresh_token   TEXT,                      -- Encrypted at rest
    token_expires   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    UNIQUE(provider, provider_uid),
    UNIQUE(user_id, provider)
);

CREATE INDEX idx_auth_providers_user ON user_auth_providers (user_id);

-- Sessions (for JWT refresh / device tracking)
CREATE TABLE sessions (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_name     TEXT,                      -- "iPhone 15", "Chrome on Linux"
    ip_hash         TEXT,                      -- Hashed IP, deleted after 48h by cron
    last_active     TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_sessions_user ON sessions (user_id);
CREATE INDEX idx_sessions_expires ON sessions (expires_at);

-- Two-factor authentication
CREATE TABLE user_2fa (
    user_id         BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    method          TEXT NOT NULL CHECK (method IN ('totp', 'webauthn')),
    totp_secret     TEXT,                      -- Encrypted at rest
    webauthn_cred   JSONB,                     -- WebAuthn credential data
    backup_codes    TEXT[],                    -- Hashed backup codes
    enabled_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================
-- SOCIAL GRAPH
-- ============================================================

-- Follows (asymmetric by default)
CREATE TABLE follows (
    follower_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    following_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status          TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'pending')),
    -- 'pending' = awaiting approval (for private accounts)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    PRIMARY KEY (follower_id, following_id),
    CONSTRAINT no_self_follow CHECK (follower_id != following_id)
);

CREATE INDEX idx_follows_following ON follows (following_id, status);
CREATE INDEX idx_follows_follower ON follows (follower_id, status);

-- Blocks
CREATE TABLE blocks (
    blocker_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    PRIMARY KEY (blocker_id, blocked_id)
);

-- ============================================================
-- POSTS & CONTENT
-- ============================================================

CREATE TABLE posts (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content         TEXT,                      -- Markdown text, max 5000 chars
    post_type       TEXT NOT NULL DEFAULT 'post' CHECK (post_type IN (
                        'post', 'story', 'video'
                    )),
    visibility      TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN (
                        'public',              -- Visible to everyone
                        'followers',           -- Only followers
                        'mentioned'            -- Only mentioned users (DM-like)
                    )),
    reply_to_id     BIGINT REFERENCES posts(id) ON DELETE SET NULL,
    thread_root_id  BIGINT REFERENCES posts(id) ON DELETE SET NULL,
    reply_count     INTEGER NOT NULL DEFAULT 0,
    -- No public like count! Only author can see how many people liked.
    like_count      INTEGER NOT NULL DEFAULT 0,
    is_edited       BOOLEAN DEFAULT false,
    expires_at      TIMESTAMPTZ,               -- For stories (24h auto-delete)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ,
    
    CONSTRAINT content_not_empty CHECK (
        content IS NOT NULL OR 
        EXISTS (SELECT 1 FROM media WHERE media.post_id = posts.id)
        -- Enforced in application layer since CHECK can't subquery
    )
);

CREATE INDEX idx_posts_user ON posts (user_id, created_at DESC);
CREATE INDEX idx_posts_reply ON posts (reply_to_id) WHERE reply_to_id IS NOT NULL;
CREATE INDEX idx_posts_thread ON posts (thread_root_id) WHERE thread_root_id IS NOT NULL;
CREATE INDEX idx_posts_expires ON posts (expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_posts_created ON posts (created_at DESC);

-- Media (images, videos)
CREATE TABLE media (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id         BIGINT REFERENCES posts(id) ON DELETE CASCADE,
    media_type      TEXT NOT NULL CHECK (media_type IN ('image', 'video')),
    storage_key     TEXT NOT NULL,             -- S3/MinIO object key
    mime_type       TEXT NOT NULL,
    file_size       BIGINT NOT NULL,           -- bytes
    width           INTEGER,
    height          INTEGER,
    duration_ms     INTEGER,                   -- For video
    blurhash        TEXT,                      -- Placeholder blur hash
    alt_text        TEXT,                      -- Accessibility
    -- All EXIF data is stripped before storage. These fields are never populated.
    processing_status TEXT DEFAULT 'pending' CHECK (processing_status IN (
                        'pending', 'processing', 'completed', 'failed'
                    )),
    variants        JSONB DEFAULT '{}',        -- {"thumb": "key", "medium": "key", "full": "key"}
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_media_post ON media (post_id) WHERE post_id IS NOT NULL;
CREATE INDEX idx_media_user ON media (user_id);
CREATE INDEX idx_media_processing ON media (processing_status) WHERE processing_status != 'completed';

-- Likes (private — only visible to post author)
CREATE TABLE likes (
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id         BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    PRIMARY KEY (user_id, post_id)
);

CREATE INDEX idx_likes_post ON likes (post_id);

-- Bookmarks (fully private, only user can see)
CREATE TABLE bookmarks (
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id         BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    PRIMARY KEY (user_id, post_id)
);

-- ============================================================
-- DIRECT MESSAGES (E2EE)
-- ============================================================

-- DM conversations
CREATE TABLE conversations (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,
    conversation_type TEXT NOT NULL DEFAULT 'direct' CHECK (conversation_type IN ('direct', 'group')),
    title           TEXT,                      -- For group chats
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE conversation_members (
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('member', 'admin')),
    last_read_at    TIMESTAMPTZ,
    muted           BOOLEAN DEFAULT false,
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    PRIMARY KEY (conversation_id, user_id)
);

CREATE INDEX idx_conv_members_user ON conversation_members (user_id);

-- Messages are E2EE: server stores ciphertext only
CREATE TABLE messages (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- E2EE: server only stores encrypted payload
    ciphertext      BYTEA NOT NULL,            -- Encrypted message content
    -- Protocol metadata (needed for decryption, not content)
    protocol_version TEXT NOT NULL DEFAULT 'mls-1.0',
    epoch           INTEGER,                   -- MLS epoch
    content_type    TEXT DEFAULT 'text',        -- 'text', 'image', 'video' (encrypted regardless)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ                -- Sender can delete for everyone
);

CREATE INDEX idx_messages_conv ON messages (conversation_id, created_at DESC);
CREATE INDEX idx_messages_sender ON messages (sender_id);

-- E2EE key packages (for MLS protocol)
CREATE TABLE key_packages (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_package     BYTEA NOT NULL,
    uploaded_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    consumed        BOOLEAN DEFAULT false
);

CREATE INDEX idx_key_packages_user ON key_packages (user_id, consumed);

-- ============================================================
-- GROUPS
-- ============================================================

CREATE TABLE groups (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,
    name            TEXT NOT NULL,
    slug            CITEXT NOT NULL UNIQUE,
    description     TEXT DEFAULT '',
    avatar_url      TEXT,
    banner_url      TEXT,
    visibility      TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'private')),
    created_by      BIGINT NOT NULL REFERENCES users(id),
    member_count    INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE group_members (
    group_id        BIGINT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('member', 'moderator', 'admin')),
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    PRIMARY KEY (group_id, user_id)
);

CREATE INDEX idx_group_members_user ON group_members (user_id);

-- Group posts use the same posts table with a group_id FK
ALTER TABLE posts ADD COLUMN group_id BIGINT REFERENCES groups(id) ON DELETE CASCADE;
CREATE INDEX idx_posts_group ON posts (group_id, created_at DESC) WHERE group_id IS NOT NULL;

-- ============================================================
-- EVENTS
-- ============================================================

CREATE TABLE events (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    group_id        BIGINT REFERENCES groups(id) ON DELETE CASCADE,
    title           TEXT NOT NULL,
    description     TEXT DEFAULT '',
    location_name   TEXT,
    location_url    TEXT,                      -- Optional map/directions link
    starts_at       TIMESTAMPTZ NOT NULL,
    ends_at         TIMESTAMPTZ,
    max_attendees   INTEGER,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE event_rsvps (
    event_id        BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status          TEXT NOT NULL CHECK (status IN ('going', 'interested', 'not_going')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    PRIMARY KEY (event_id, user_id)
);

-- ============================================================
-- NOTIFICATIONS
-- ============================================================

CREATE TABLE notifications (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type            TEXT NOT NULL CHECK (type IN (
                        'follow', 'follow_request',
                        'like', 'reply', 'mention',
                        'group_invite', 'event_invite',
                        'dm'
                    )),
    actor_id        BIGINT REFERENCES users(id) ON DELETE CASCADE,
    post_id         BIGINT REFERENCES posts(id) ON DELETE CASCADE,
    group_id        BIGINT REFERENCES groups(id) ON DELETE CASCADE,
    event_id        BIGINT REFERENCES events(id) ON DELETE CASCADE,
    read            BOOLEAN DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_notifications_user ON notifications (user_id, read, created_at DESC);

-- ============================================================
-- SUGGESTIONS (mutual friends only)
-- ============================================================

-- Materialized view: "People you may know" based ONLY on mutual follows
-- No behavioral data, no content analysis, no profiling.
CREATE MATERIALIZED VIEW suggested_connections AS
WITH mutual_counts AS (
    SELECT 
        f1.follower_id AS user_a,
        f2.following_id AS user_b,
        COUNT(*) AS mutual_count
    FROM follows f1
    JOIN follows f2 ON f1.following_id = f2.follower_id
    WHERE f1.follower_id != f2.following_id
      AND f1.status = 'active'
      AND f2.status = 'active'
      -- Exclude already following
      AND NOT EXISTS (
          SELECT 1 FROM follows f3 
          WHERE f3.follower_id = f1.follower_id 
            AND f3.following_id = f2.following_id
      )
      -- Exclude blocked
      AND NOT EXISTS (
          SELECT 1 FROM blocks b 
          WHERE (b.blocker_id = f1.follower_id AND b.blocked_id = f2.following_id)
             OR (b.blocker_id = f2.following_id AND b.blocked_id = f1.follower_id)
      )
    GROUP BY f1.follower_id, f2.following_id
    HAVING COUNT(*) >= 2  -- At least 2 mutual connections
)
SELECT user_a, user_b, mutual_count
FROM mutual_counts
ORDER BY user_a, mutual_count DESC;

-- Refresh nightly via cron (not real-time — privacy conscious)
-- REFRESH MATERIALIZED VIEW CONCURRENTLY suggested_connections;

CREATE UNIQUE INDEX idx_suggested_user ON suggested_connections (user_a, user_b);

-- ============================================================
-- FINANCES (Open Boekhouding)
-- ============================================================

CREATE TABLE finances (
    id              BIGSERIAL PRIMARY KEY,
    date            DATE NOT NULL,
    type            TEXT NOT NULL CHECK (type IN (
                        'donation_github',
                        'donation_bmac',
                        'donation_inapp',
                        'expense_hosting',
                        'expense_service',
                        'expense_domain',
                        'expense_security_audit',
                        'expense_transaction_fee',
                        'charity_payout',
                        'reserve_transfer'
                    )),
    amount_cents    INTEGER NOT NULL,          -- Positive = income, negative = expense
    currency        TEXT NOT NULL DEFAULT 'EUR',
    description     TEXT NOT NULL,
    donor_name      TEXT,                      -- NULL = anonymous
    donor_public    BOOLEAN DEFAULT false,     -- May name be shown publicly?
    recipient       TEXT,                      -- Supplier or charity name
    receipt_url     TEXT,                      -- Link to invoice/proof
    category        TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_finances_date ON finances (date DESC);
CREATE INDEX idx_finances_type ON finances (type);

-- Monthly summary (auto-refreshed)
CREATE MATERIALIZED VIEW finance_monthly AS
SELECT 
    date_trunc('month', date)::DATE AS month,
    SUM(CASE WHEN amount_cents > 0 THEN amount_cents ELSE 0 END) AS income_cents,
    SUM(CASE WHEN amount_cents < 0 THEN abs(amount_cents) ELSE 0 END) AS expense_cents,
    SUM(amount_cents) AS net_cents,
    COUNT(*) FILTER (WHERE type LIKE 'donation_%') AS donation_count,
    COUNT(*) FILTER (WHERE type LIKE 'expense_%') AS expense_count
FROM finances
GROUP BY 1
ORDER BY 1 DESC;

CREATE UNIQUE INDEX idx_finance_monthly ON finance_monthly (month);

-- Charity payouts
CREATE TABLE charity_payouts (
    id              BIGSERIAL PRIMARY KEY,
    year            INTEGER NOT NULL,
    charity_name    TEXT NOT NULL,
    charity_url     TEXT,
    charity_country TEXT,
    amount_cents    INTEGER NOT NULL,
    vote_percentage NUMERIC(5,2),
    payment_proof   TEXT,                      -- URL to payment receipt
    paid_at         DATE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Community voting on charity distribution
CREATE TABLE charity_candidates (
    id              BIGSERIAL PRIMARY KEY,
    year            INTEGER NOT NULL,
    charity_name    TEXT NOT NULL,
    charity_url     TEXT,
    description     TEXT,
    pillar          TEXT NOT NULL CHECK (pillar IN ('peace', 'health')),
    nominated_by    BIGINT REFERENCES users(id),
    approved        BOOLEAN DEFAULT false,     -- Core team approves shortlist
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    UNIQUE(year, charity_name)
);

CREATE TABLE charity_votes (
    id              BIGSERIAL PRIMARY KEY,
    year            INTEGER NOT NULL,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    candidate_id    BIGINT NOT NULL REFERENCES charity_candidates(id) ON DELETE CASCADE,
    points          INTEGER NOT NULL CHECK (points >= 0 AND points <= 100),
    voted_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    UNIQUE(year, user_id, candidate_id)
);

-- Ensure max 100 points per user per year (enforced in app + trigger)
CREATE OR REPLACE FUNCTION check_vote_points() RETURNS TRIGGER AS $$
BEGIN
    IF (
        SELECT COALESCE(SUM(points), 0) + NEW.points 
        FROM charity_votes 
        WHERE year = NEW.year AND user_id = NEW.user_id AND id != COALESCE(NEW.id, 0)
    ) > 100 THEN
        RAISE EXCEPTION 'Maximum 100 vote points per year';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_vote_points 
    BEFORE INSERT OR UPDATE ON charity_votes
    FOR EACH ROW EXECUTE FUNCTION check_vote_points();

-- ============================================================
-- REPORTS & MODERATION
-- ============================================================

CREATE TABLE reports (
    id              BIGSERIAL PRIMARY KEY,
    reporter_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reported_user_id BIGINT REFERENCES users(id) ON DELETE CASCADE,
    reported_post_id BIGINT REFERENCES posts(id) ON DELETE SET NULL,
    reason          TEXT NOT NULL CHECK (reason IN (
                        'spam', 'harassment', 'hate_speech',
                        'violence', 'nudity', 'misinformation',
                        'illegal_content', 'other'
                    )),
    description     TEXT,
    status          TEXT NOT NULL DEFAULT 'pending' CHECK (status IN (
                        'pending', 'reviewing', 'resolved', 'dismissed'
                    )),
    resolved_by     BIGINT REFERENCES users(id),
    resolution_note TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at     TIMESTAMPTZ
);

CREATE INDEX idx_reports_status ON reports (status, created_at);

-- ============================================================
-- WAITLIST (pre-launch)
-- ============================================================

CREATE TABLE waitlist (
    id              BIGSERIAL PRIMARY KEY,
    email           CITEXT NOT NULL UNIQUE,
    source          TEXT DEFAULT 'landing',     -- 'landing', 'invite', 'github'
    invited_at      TIMESTAMPTZ,               -- When invite was sent
    registered_at   TIMESTAMPTZ,               -- When they actually signed up
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================
-- INVITES
-- ============================================================

CREATE TABLE invites (
    id              BIGSERIAL PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE DEFAULT encode(gen_random_bytes(8), 'hex'),
    created_by      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    used_by         BIGINT REFERENCES users(id),
    expires_at      TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '7 days',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================
-- SETTINGS & PREFERENCES
-- ============================================================

CREATE TABLE user_settings (
    user_id             BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    
    -- Privacy
    show_online_status  BOOLEAN DEFAULT false,
    show_read_receipts  BOOLEAN DEFAULT false,
    allow_dms_from      TEXT DEFAULT 'followers' CHECK (allow_dms_from IN ('everyone', 'followers', 'nobody')),
    discoverable        BOOLEAN DEFAULT true,  -- Show in "people you may know"
    
    -- Notifications
    notify_follows      BOOLEAN DEFAULT true,
    notify_likes        BOOLEAN DEFAULT false,  -- Off by default (reduce dopamine)
    notify_replies      BOOLEAN DEFAULT true,
    notify_dms          BOOLEAN DEFAULT true,
    notify_email        BOOLEAN DEFAULT false,
    
    -- Content
    default_visibility  TEXT DEFAULT 'public' CHECK (default_visibility IN ('public', 'followers')),
    
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================
-- MAINTENANCE
-- ============================================================

-- Auto-delete expired stories
CREATE OR REPLACE FUNCTION cleanup_expired_posts() RETURNS void AS $$
BEGIN
    DELETE FROM posts WHERE expires_at IS NOT NULL AND expires_at < now();
END;
$$ LANGUAGE plpgsql;

-- Auto-delete old IP hashes (privacy: max 48h retention)
CREATE OR REPLACE FUNCTION cleanup_ip_hashes() RETURNS void AS $$
BEGIN
    UPDATE sessions SET ip_hash = NULL 
    WHERE ip_hash IS NOT NULL 
      AND last_active < now() - INTERVAL '48 hours';
END;
$$ LANGUAGE plpgsql;

-- Auto-delete expired sessions
CREATE OR REPLACE FUNCTION cleanup_expired_sessions() RETURNS void AS $$
BEGIN
    DELETE FROM sessions WHERE expires_at < now();
END;
$$ LANGUAGE plpgsql;

-- ============================================================
-- CITUS PREPARATION
-- ============================================================
-- When ready to distribute, uncomment these:
--
-- CREATE EXTENSION citus;
--
-- SELECT create_distributed_table('users', 'id');
-- SELECT create_distributed_table('posts', 'user_id');
-- SELECT create_distributed_table('follows', 'follower_id');
-- SELECT create_distributed_table('likes', 'user_id');
-- SELECT create_distributed_table('bookmarks', 'user_id');
-- SELECT create_distributed_table('media', 'user_id');
-- SELECT create_distributed_table('messages', 'sender_id');
-- SELECT create_distributed_table('notifications', 'user_id');
-- SELECT create_distributed_table('sessions', 'user_id');
--
-- SELECT create_reference_table('groups');
-- SELECT create_reference_table('events');
-- SELECT create_reference_table('finances');
-- SELECT create_reference_table('charity_candidates');
-- SELECT create_reference_table('charity_payouts');
