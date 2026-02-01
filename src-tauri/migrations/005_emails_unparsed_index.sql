-- get_unparsed_emails クエリのパフォーマンス改善
--
-- 本インデックス: body_plain, from_address が NULL でない行に限定し、ORDER BY internal_date を効率化。
-- oe.email_id IS NULL / ps.email_id IS NULL の判定は LEFT JOIN で行われ、
-- order_emails(email_id) は idx_order_emails_email_id（001）、parse_skipped(email_id) は PK で既に最適化済み。
CREATE INDEX IF NOT EXISTS idx_emails_unparsed_filter
ON emails(internal_date)
WHERE body_plain IS NOT NULL AND from_address IS NOT NULL;
