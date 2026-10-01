-- Migration 0003: Add attachments table and attachment fields to expenses

-- 1. Add attachment columns to expenses
ALTER TABLE expenses ADD COLUMN attachment_url TEXT;
ALTER TABLE expenses ADD COLUMN attachment_name TEXT;
ALTER TABLE expenses ADD COLUMN attachment_id TEXT;

-- 2. Create attachments table
CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY NOT NULL,
    expense_id TEXT REFERENCES expenses(id) ON DELETE SET NULL,
    group_id TEXT REFERENCES groups(id) ON DELETE CASCADE,
    uploaded_by TEXT NOT NULL REFERENCES users(id),
    file_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    content_type TEXT NOT NULL,
    ocr_text TEXT,
    detected_amount_cents INTEGER,
    detected_merchant TEXT,
    created_at TEXT NOT NULL
);

-- 3. Create indexes
CREATE INDEX IF NOT EXISTS idx_attachments_expense_id ON attachments(expense_id);
CREATE INDEX IF NOT EXISTS idx_attachments_group_id ON attachments(group_id);
