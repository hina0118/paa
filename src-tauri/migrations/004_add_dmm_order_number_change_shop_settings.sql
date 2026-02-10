-- DMM通販 配送センター変更に伴う注文番号変更メール用パーサー設定を追加
INSERT OR IGNORE INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('DMM通販', 'info@mail.dmm.com', 'dmm_order_number_change', '["DMM通販：配送センター変更に伴うご注文番号変更のお知らせ"]', 1);
