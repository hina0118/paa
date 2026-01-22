-- Add subject column to emails table
ALTER TABLE emails ADD COLUMN subject TEXT;

-- Create index for subject lookup
CREATE INDEX idx_emails_subject ON emails(subject);
