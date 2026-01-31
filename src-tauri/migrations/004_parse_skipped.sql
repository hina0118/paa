-- パース失敗したメールを記録し、無限ループを防ぐ
-- order_emails に含まれない失敗メールが毎回 get_unparsed_emails で返され続けるのを防止
CREATE TABLE IF NOT EXISTS parse_skipped (
    email_id INTEGER PRIMARY KEY,
    error_message TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_parse_skipped_email_id ON parse_skipped(email_id);
