-- Add batch_size column to parse_metadata table
ALTER TABLE parse_metadata ADD COLUMN batch_size INTEGER NOT NULL DEFAULT 100;
