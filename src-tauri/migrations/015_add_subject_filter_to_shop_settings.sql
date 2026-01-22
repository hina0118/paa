-- Add subject_filters column to shop_settings table for filtering emails by subject
-- Stores JSON array of subject patterns
ALTER TABLE shop_settings ADD COLUMN subject_filters TEXT;

-- Insert default shop settings for ホビーサーチ with subject filters (JSON array)
INSERT INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('ホビーサーチ', 'hs-support@1999.co.jp', 'hobbysearch', '["【ホビーサーチ】ご注文の発送が完了しました","【ホビーサーチ】ご注文が組み替えられました"]', 1);
INSERT INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('ホビーサーチ', 'hs-order@1999.co.jp', 'hobbysearch', '["【ホビーサーチ】注文確認メール"]', 1);
