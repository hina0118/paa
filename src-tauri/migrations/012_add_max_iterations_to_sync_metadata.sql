-- Add max_iterations column to sync_metadata table
ALTER TABLE sync_metadata ADD COLUMN max_iterations INTEGER NOT NULL DEFAULT 1000;
