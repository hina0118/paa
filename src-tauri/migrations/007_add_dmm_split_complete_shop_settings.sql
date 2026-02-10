-- 007: DMM通販「ご注文分割完了のお知らせ」用パーサー設定
INSERT OR IGNORE INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('DMM通販', 'info@mail.dmm.com', 'dmm_split_complete', '["DMM通販：ご注文分割完了のお知らせ"]', 1);
