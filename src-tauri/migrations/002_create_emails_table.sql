-- Create emails table for Gmail message storage
CREATE TABLE IF NOT EXISTS emails (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id TEXT UNIQUE NOT NULL,
    body_plain TEXT,
    body_html TEXT,
    analysis_status TEXT NOT NULL DEFAULT 'pending' CHECK(analysis_status IN ('pending', 'completed')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Index for message_id lookup
CREATE INDEX idx_emails_message_id ON emails(message_id);

-- Index for filtering by analysis status
CREATE INDEX idx_emails_analysis_status ON emails(analysis_status);
