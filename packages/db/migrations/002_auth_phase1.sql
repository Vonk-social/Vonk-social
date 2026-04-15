-- ============================================================
-- VONK — Phase 1 auth additions
-- ============================================================
-- Adds onboarding tracking on the existing users table.
-- NULL onboarding_completed_at == user still in the 3-step wizard.
-- ============================================================

ALTER TABLE users
    ADD COLUMN onboarding_completed_at TIMESTAMPTZ;

-- Partial index: most lookups for users-still-in-onboarding filter on NULL.
CREATE INDEX idx_users_onboarding_pending
    ON users (created_at)
    WHERE onboarding_completed_at IS NULL;
