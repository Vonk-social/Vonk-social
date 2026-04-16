-- Avatar perceptual hash for duplicate detection.
-- Prevents fake accounts from reusing another user's profile photo.
ALTER TABLE users ADD COLUMN avatar_phash BIGINT;
CREATE INDEX idx_users_avatar_phash ON users (avatar_phash) WHERE avatar_phash IS NOT NULL;
