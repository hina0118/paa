-- orders に shop_name を追加（表示用、shop_settings.shop_name から取得）
ALTER TABLE orders ADD COLUMN shop_name TEXT;
