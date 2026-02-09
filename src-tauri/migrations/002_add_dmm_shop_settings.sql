-- DMM通販 ご注文手続き完了メール用パーサー設定を追加
INSERT OR IGNORE INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('DMM通販', 'info@mail.dmm.com', 'dmm_confirm', '["DMM通販：ご注文手続き完了のお知らせ"]', 1);
