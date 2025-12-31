-- Add password_updated_at column to track when password was last changed
-- This enables showing "Last changed X days ago" in account settings

ALTER TABLE users ADD COLUMN password_updated_at TIMESTAMPTZ;

-- Set initial value to created_at for existing users (password was set at account creation)
UPDATE users SET password_updated_at = created_at WHERE password_updated_at IS NULL;

-- Make the column NOT NULL with a default for future inserts
ALTER TABLE users ALTER COLUMN password_updated_at SET NOT NULL;
ALTER TABLE users ALTER COLUMN password_updated_at SET DEFAULT NOW();

-- Add index for potential queries filtering by password age (security reports, etc.)
CREATE INDEX idx_users_password_updated_at ON users(password_updated_at);
