-- Add from_address column to emails table
ALTER TABLE emails ADD COLUMN from_address TEXT;

-- Create index for from_address lookup
CREATE INDEX idx_emails_from_address ON emails(from_address);
