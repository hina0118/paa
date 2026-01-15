-- Add internal_date column to emails table for chronological tracking
ALTER TABLE emails ADD COLUMN internal_date INTEGER;

-- Index for efficient date-based queries
CREATE INDEX idx_emails_internal_date ON emails(internal_date);
