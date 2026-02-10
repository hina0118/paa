-- 006: items_fts を trigram トークナイザーに変更
-- trigram により「ンダム」→「ガンダム」のような部分一致検索が可能に
-- 要件: SQLite 3.43 以降（trigram トークナイザーは 3.43 で追加）

-- 1. トリガー削除
DROP TRIGGER IF EXISTS items_fts_insert;
DROP TRIGGER IF EXISTS items_fts_update;
DROP TRIGGER IF EXISTS items_fts_delete;

-- 2. 既存の items_fts 削除
DROP TABLE IF EXISTS items_fts;

-- 3. trigram トークナイザーで再作成
CREATE VIRTUAL TABLE items_fts USING fts5(
    item_name,
    item_name_normalized,
    brand,
    category,
    content=items,
    content_rowid=id,
    tokenize='trigram'
);

-- 4. トリガー再作成
CREATE TRIGGER items_fts_insert AFTER INSERT ON items BEGIN
    INSERT INTO items_fts(rowid, item_name, item_name_normalized, brand, category)
    VALUES (new.id, new.item_name, new.item_name_normalized, new.brand, new.category);
END;
CREATE TRIGGER items_fts_update AFTER UPDATE ON items BEGIN
    UPDATE items_fts
    SET item_name = new.item_name,
        item_name_normalized = new.item_name_normalized,
        brand = new.brand,
        category = new.category
    WHERE rowid = new.id;
END;
CREATE TRIGGER items_fts_delete AFTER DELETE ON items BEGIN
    DELETE FROM items_fts WHERE rowid = old.id;
END;

-- 5. 既存データを FTS インデックスに再構築
INSERT INTO items_fts(items_fts) VALUES('rebuild');
