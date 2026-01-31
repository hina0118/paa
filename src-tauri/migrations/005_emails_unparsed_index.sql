-- get_unparsed_emails クエリのパフォーマンス改善
-- body_plain, from_address が NULL でない行に限定し、ORDER BY internal_date を効率化
CREATE INDEX IF NOT EXISTS idx_emails_unparsed_filter
ON emails(internal_date)
WHERE body_plain IS NOT NULL AND from_address IS NOT NULL;
