-- Migration 0002: Add created_by to expenses and create activity_logs table

-- 1. Add created_by column to expenses
ALTER TABLE expenses ADD COLUMN created_by TEXT REFERENCES users(id);

-- 2. Backfill created_by with paid_by for existing expense rows
UPDATE expenses SET created_by = paid_by WHERE created_by IS NULL;

-- 3. Create activity_logs table for group audit trails
CREATE TABLE IF NOT EXISTS activity_logs (
    id TEXT PRIMARY KEY NOT NULL,
    group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id),
    action TEXT NOT NULL,
    target_id TEXT,
    summary TEXT NOT NULL,
    details TEXT,
    created_at TEXT NOT NULL
);

-- 4. Create index for fast chronological group activity lookup
CREATE INDEX IF NOT EXISTS idx_activity_logs_group ON activity_logs(group_id, created_at DESC);
