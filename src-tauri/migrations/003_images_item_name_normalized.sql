-- 003_images_item_name_normalized: imagesテーブルにitem_name_normalizedカラムを追加
--
-- 目的: パース再実行時にも画像を維持するため、item_idではなく正規化商品名で関連付ける
-- item_idはパース再実行時に変わる可能性があるが、item_name_normalizedは商品名が同じなら同一値

-- 正規化商品名カラムを追加
ALTER TABLE images ADD COLUMN item_name_normalized TEXT;

-- 既存データの移行: itemsテーブルから正規化商品名をコピー
UPDATE images
SET item_name_normalized = (
    SELECT i.item_name_normalized
    FROM items i
    WHERE i.id = images.item_id
)
WHERE item_name_normalized IS NULL;

-- 正規化商品名でのインデックスを作成（LEFT JOIN高速化用）
CREATE INDEX IF NOT EXISTS idx_images_item_name_normalized ON images(item_name_normalized);
