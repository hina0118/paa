-- 010: DMM通販 手続き完了・発送メールの件名フィルタをゆるくする
-- 「DMM通販：」が付かない件名パターンも拾えるように、subject_filters を複数パターンに更新する。

-- ご注文手続き完了メール
UPDATE shop_settings
SET subject_filters = '["DMM通販：ご注文手続き完了のお知らせ", "ご注文手続き完了のお知らせ"]'
WHERE sender_address IN ('info@mail.dmm.com', 'info@mono.dmm.com')
  AND parser_type = 'dmm_confirm';

-- ご注文商品を発送いたしましたメール
UPDATE shop_settings
SET subject_filters = '["DMM通販：ご注文商品を発送いたしました", "ご注文商品を発送いたしました"]'
WHERE sender_address IN ('info@mail.dmm.com', 'info@mono.dmm.com')
  AND parser_type = 'dmm_send';

