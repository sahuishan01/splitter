-- Migration 0004: Add system_settings table and must_change_password flag to users

-- 1. Add must_change_password column to users table
ALTER TABLE users ADD COLUMN must_change_password BOOLEAN NOT NULL DEFAULT 0;

-- 2. Create system_settings table for dynamic runtime configurations
CREATE TABLE IF NOT EXISTS system_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 3. Default allow_signup to true
INSERT OR IGNORE INTO system_settings (key, value, updated_at)
VALUES ('allow_signup', 'true', datetime('now'));
