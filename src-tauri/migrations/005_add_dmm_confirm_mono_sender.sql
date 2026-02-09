-- DMM通販 注文手続き完了メール: info@mono.dmm.com からの送信にも対応
-- （注文手続き完了は info@mail.dmm.com と info@mono.dmm.com の両方から届く場合がある）
INSERT OR IGNORE INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('DMM通販', 'info@mono.dmm.com', 'dmm_confirm', '["DMM通販:ご注文手続き完了のお知らせ"]', 1);
