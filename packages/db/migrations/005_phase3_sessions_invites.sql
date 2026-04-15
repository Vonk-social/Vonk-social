-- ============================================================
-- VONK — Phase 3 foundation
-- ============================================================
-- Adds columns + tables used across Phase 3:
--   - sessions.rotated_to: set when a refresh token is redeemed for a
--     new one; reusing the old (rotated) token triggers "reuse detected"
--     and invalidates the whole chain.
--   - messages: encryption_version, ephemeral_pubkey, nonce for E2EE v1
--     (AES-256-GCM + X25519 ECDH). Existing plaintext rows stay at v0.
--   - users: public_key (X25519 base64), social handles (IG / X / Snap /
--     Telegram / Bluesky / Mastodon + generic site).
--   - email_invites: outgoing invite audit log so a recipient isn't
--     spammed, and we can tell what's landed.
--   - push_subscriptions: Web Push endpoints per session/device.
--   - user_contact_hashes: staged for Capacitor contacts sync (hashed
--     email + phone, Phase 3.1 will populate).
-- ============================================================

-- ── Sessions: refresh-token rotation ────────────────────────
ALTER TABLE sessions
    ADD COLUMN rotated_to UUID REFERENCES sessions(id) ON DELETE SET NULL,
    ADD COLUMN rotated_at TIMESTAMPTZ;

CREATE INDEX idx_sessions_rotated_to ON sessions (rotated_to)
    WHERE rotated_to IS NOT NULL;

-- ── Messages: E2EE v1 envelope ──────────────────────────────
ALTER TABLE messages
    ADD COLUMN encryption_version SMALLINT NOT NULL DEFAULT 0,
    -- Sender's ephemeral X25519 public key for this message (32 bytes).
    -- Needed to reconstruct the shared secret on decrypt.
    ADD COLUMN ephemeral_pubkey BYTEA,
    -- AES-GCM 12-byte nonce. Generated per message.
    ADD COLUMN nonce BYTEA;

CREATE INDEX idx_messages_encryption ON messages (encryption_version)
    WHERE encryption_version > 0;

-- ── Users: public key + external social handles ─────────────
ALTER TABLE users
    -- X25519 public key, base64-url, 43 chars + no padding.
    -- Null until the user registers a keypair (on first E2EE snap send
    -- or via /settings). Unregistered users can still receive plaintext
    -- snaps (v0) but not v1 encrypted snaps.
    ADD COLUMN public_key TEXT,
    ADD COLUMN handle_instagram TEXT,
    ADD COLUMN handle_twitter  TEXT,
    ADD COLUMN handle_snapchat TEXT,
    ADD COLUMN handle_telegram TEXT,
    ADD COLUMN handle_bluesky  TEXT,
    ADD COLUMN handle_mastodon TEXT,
    ADD COLUMN handle_website  TEXT;

-- ── Email invites (Postal SMTP) ─────────────────────────────
CREATE TABLE email_invites (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,
    sender_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Address as entered by sender. Compared case-insensitively.
    recipient_email CITEXT NOT NULL,
    -- Personal note shown in the email body (optional).
    note            TEXT,
    -- Becomes non-null once Postal accepts the email; null while queued
    -- or after a permanent failure.
    sent_at         TIMESTAMPTZ,
    -- If recipient accepts and signs up, their user_id lands here.
    accepted_by_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    accepted_at     TIMESTAMPTZ,
    -- Permanent failure (postal rejected, bounced, etc.)
    failed_at       TIMESTAMPTZ,
    failure_reason  TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One outstanding invite per (sender, recipient). A second call is a
-- no-op rather than a dupe.
CREATE UNIQUE INDEX uq_email_invites_sender_recipient
    ON email_invites (sender_id, recipient_email)
    WHERE accepted_at IS NULL AND failed_at IS NULL;

CREATE INDEX idx_email_invites_recipient ON email_invites (recipient_email);
CREATE INDEX idx_email_invites_pending ON email_invites (created_at)
    WHERE sent_at IS NULL AND failed_at IS NULL;

-- ── Web Push subscriptions ──────────────────────────────────
CREATE TABLE push_subscriptions (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- VAPID endpoint URL (varies per browser / push service).
    endpoint        TEXT NOT NULL,
    p256dh          TEXT NOT NULL,
    auth            TEXT NOT NULL,
    -- "web" / "apns" / "fcm"; Phase 3 ships "web" only.
    kind            TEXT NOT NULL DEFAULT 'web',
    -- Free-text, from the browser UA. Purely cosmetic ("Chrome on Linux").
    user_agent      TEXT,
    -- User-controlled flags, toggled in /settings.
    notify_dm       BOOLEAN NOT NULL DEFAULT true,
    notify_mention  BOOLEAN NOT NULL DEFAULT true,
    notify_follow   BOOLEAN NOT NULL DEFAULT true,
    notify_reply    BOOLEAN NOT NULL DEFAULT false,
    last_delivery_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Browsers re-issue endpoints — unique per (user, endpoint) prevents
-- duplicates if the user opts in twice on the same device.
CREATE UNIQUE INDEX uq_push_user_endpoint
    ON push_subscriptions (user_id, md5(endpoint));

-- ── Contacts hashes (Capacitor-driven Phase 3.1) ────────────
-- Staged: we don't populate this yet but the schema is ready so the
-- mobile plugin can upload directly. `hash` is sha256(lower(email)|salt)
-- or sha256(E.164 phone|salt), salt is shared (not per-user).
CREATE TABLE user_contact_hashes (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    hash            BYTEA NOT NULL,
    kind            TEXT NOT NULL CHECK (kind IN ('email', 'phone')),
    matched_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_contact_hashes_hash ON user_contact_hashes (hash);
CREATE INDEX idx_contact_hashes_owner ON user_contact_hashes (user_id);
