-- DMM通販 ご注文キャンセルメール用パーサー設定を追加
INSERT OR IGNORE INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('DMM通販', 'info@mail.dmm.com', 'dmm_cancel', '["DMM通販：ご注文キャンセルのお知らせ"]', 1);
