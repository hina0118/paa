-- 020: images を BLOB → file_name に変更 (issue #47)
CREATE TABLE images_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id INTEGER NOT NULL,
    file_name TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE,
    UNIQUE (item_id)
);
CREATE UNIQUE INDEX idx_images_new_item_id ON images_new(item_id);

-- 既存行は移行しない（image_data は破棄）
DROP TABLE images;
ALTER TABLE images_new RENAME TO images;
-- インデックス名を元に戻すため再作成
DROP INDEX idx_images_new_item_id;
CREATE UNIQUE INDEX idx_images_item_id ON images(item_id);
