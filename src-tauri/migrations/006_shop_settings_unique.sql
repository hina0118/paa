-- 既存インストール用: shop_settings に UNIQUE(sender_address, parser_type) を追加
-- 001_init を後から修正した新規インストールでは 001 で既に UNIQUE を持つ
--
-- 重複を削除してから UNIQUE インデックスを作成（id が小さい行を残す）
DELETE FROM shop_settings
WHERE id IN (
  SELECT s1.id FROM shop_settings s1
  WHERE EXISTS (
    SELECT 1 FROM shop_settings s2
    WHERE s1.sender_address = s2.sender_address
      AND s1.parser_type = s2.parser_type
      AND s1.id > s2.id
  )
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_shop_settings_sender_parser
ON shop_settings(sender_address, parser_type);
