-- ============================================================
-- VONK — Phase 4: ActivityPub federation
-- ============================================================
-- Makes Vonk users discoverable and followable from Mastodon,
-- Pixelfed, and other fediverse platforms via the ActivityPub
-- protocol.
--
-- This migration adds:
--   - ap_pubkey / ap_privkey on users (RSA keypair for HTTP Signatures)
--   - ap_followers (remote fediverse followers)
--   - ap_remote_actors (cache of remote actor profiles)
--   - ap_delivery_queue (outgoing activity delivery)
--   - ap_remote_posts (federated content for future feed integration)
-- ============================================================

-- ── RSA keypair on users (for HTTP Signatures) ─────────────
ALTER TABLE users ADD COLUMN IF NOT EXISTS ap_pubkey TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS ap_privkey TEXT;

-- ── Remote actor cache ─────────────────────────────────────
-- When a remote user (e.g. @alice@mastodon.social) interacts
-- with a Vonk user, we cache their actor profile here.
CREATE TABLE ap_remote_actors (
    id              BIGSERIAL PRIMARY KEY,
    actor_uri       TEXT NOT NULL UNIQUE,       -- https://mastodon.social/users/alice
    inbox_url       TEXT NOT NULL,
    shared_inbox_url TEXT,
    public_key_pem  TEXT NOT NULL,
    username        TEXT,
    display_name    TEXT,
    avatar_url      TEXT,
    summary         TEXT,
    fetched_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_ap_remote_actors_uri ON ap_remote_actors (actor_uri);

-- ── Remote followers (fediverse users following Vonk users) ─
CREATE TABLE ap_followers (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    actor_uri       TEXT NOT NULL,              -- remote actor URI
    inbox_url       TEXT NOT NULL,              -- where to deliver activities
    shared_inbox_url TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, actor_uri)
);

CREATE INDEX idx_ap_followers_user ON ap_followers (user_id);

-- ── Delivery queue (outgoing activities) ───────────────────
CREATE TABLE ap_delivery_queue (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    inbox_url       TEXT NOT NULL,
    payload         JSONB NOT NULL,
    attempts        INT NOT NULL DEFAULT 0,
    max_attempts    INT NOT NULL DEFAULT 5,
    next_retry      TIMESTAMPTZ NOT NULL DEFAULT now(),
    delivered       BOOLEAN NOT NULL DEFAULT false,
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_ap_delivery_pending ON ap_delivery_queue (next_retry)
    WHERE NOT delivered AND attempts < max_attempts;

-- ── Remote posts (federated content for future feed) ───────
CREATE TABLE ap_remote_posts (
    id              BIGSERIAL PRIMARY KEY,
    uri             TEXT NOT NULL UNIQUE,       -- https://mastodon.social/users/alice/statuses/123
    actor_uri       TEXT NOT NULL,
    content         TEXT,
    in_reply_to     TEXT,                       -- URI of parent post (if reply)
    published_at    TIMESTAMPTZ,
    fetched_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    raw_json        JSONB NOT NULL              -- original activity JSON
);

CREATE INDEX idx_ap_remote_posts_actor ON ap_remote_posts (actor_uri);
CREATE INDEX idx_ap_remote_posts_published ON ap_remote_posts (published_at DESC);
