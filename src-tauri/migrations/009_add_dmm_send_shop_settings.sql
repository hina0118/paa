-- 009: DMM通販「ご注文商品を発送いたしました」用パーサー設定
-- info@mail.dmm.com と info@mono.dmm.com の両方から発送メールが届くため、両送信元を登録
INSERT OR IGNORE INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('DMM通販', 'info@mail.dmm.com', 'dmm_send', '["DMM通販：ご注文商品を発送いたしました"]', 1),
    ('DMM通販', 'info@mono.dmm.com', 'dmm_send', '["DMM通販：ご注文商品を発送いたしました"]', 1);

