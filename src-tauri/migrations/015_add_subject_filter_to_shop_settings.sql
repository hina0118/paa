-- Add subject_filters column to shop_settings table for filtering emails by subject
-- Stores JSON array of subject patterns
ALTER TABLE shop_settings ADD COLUMN subject_filters TEXT;

-- Insert default shop settings for ホビーサーチ with separated parsers
-- Each email type has a dedicated parser for precise extraction

-- 1. 発送通知メール用パーサー (hobbysearch_send)
INSERT INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('ホビーサーチ', 'hs-support@1999.co.jp', 'hobbysearch_send', '["【ホビーサーチ】ご注文の発送が完了しました"]', 1);

-- 2. 組み換えメール用パーサー (hobbysearch_change)
INSERT INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('ホビーサーチ', 'hs-support@1999.co.jp', 'hobbysearch_change', '["【ホビーサーチ】ご注文が組み替えられました"]', 1);

-- 3. 予約注文確認メール用パーサー (hobbysearch_confirm_yoyaku)
-- 注: 通常の注文確認と件名が同じため、先に予約用パーサーを試す
-- [ご予約内容]セクションがない場合はエラーとなり、次のパーサーを試す
INSERT INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('ホビーサーチ', 'hs-order@1999.co.jp', 'hobbysearch_confirm_yoyaku', '["【ホビーサーチ】注文確認メール"]', 1);

-- 4. 通常注文確認メール用パーサー (hobbysearch_confirm)
-- 注: 予約用パーサーが失敗した場合に試される（フォールバック）
-- [ご購入内容]セクションを探す
INSERT INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('ホビーサーチ', 'hs-order@1999.co.jp', 'hobbysearch_confirm', '["【ホビーサーチ】注文確認メール"]', 1);

-- Note: 複数のパーサーが同じ件名フィルターに一致する場合、登録順に試される
-- 予約用パーサー → 通常用パーサーの順で試すことで、適切に振り分けられる
