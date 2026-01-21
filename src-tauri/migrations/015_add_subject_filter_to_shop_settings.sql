-- Add subject_filter column to shop_settings table for filtering emails by subject
ALTER TABLE shop_settings ADD COLUMN subject_filter TEXT;

-- Insert default shop settings for ホビーサーチ with subject filter
INSERT INTO shop_settings (shop_name, sender_address, parser_type, subject_filter, is_enabled) VALUES
    ('ホビーサーチ', 'hs-support@1999.co.jp', 'hobbysearch', '【ホビーサーチ】ご注文の発送が完了しました', 1);
