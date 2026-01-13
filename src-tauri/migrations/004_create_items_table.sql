-- Create items table for product management
CREATE TABLE IF NOT EXISTS items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    item_name TEXT NOT NULL,
    item_name_normalized TEXT,
    price INTEGER NOT NULL DEFAULT 0,
    quantity INTEGER NOT NULL DEFAULT 1,
    category TEXT,
    brand TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
);

-- Index for order_id lookup
CREATE INDEX idx_items_order_id ON items(order_id);

-- Index for item_name search
CREATE INDEX idx_items_item_name ON items(item_name);

-- Index for created_at sorting
CREATE INDEX idx_items_created_at ON items(created_at DESC);

-- Index for category filtering
CREATE INDEX idx_items_category ON items(category) WHERE category IS NOT NULL;

-- Index for brand filtering
CREATE INDEX idx_items_brand ON items(brand) WHERE brand IS NOT NULL;

-- Create FTS5 virtual table for full-text search
CREATE VIRTUAL TABLE items_fts USING fts5(
    item_name,
    item_name_normalized,
    brand,
    category,
    content=items,
    content_rowid=id,
    tokenize='unicode61 remove_diacritics 2'
);

-- Trigger to insert into FTS5 table
CREATE TRIGGER items_fts_insert AFTER INSERT ON items BEGIN
    INSERT INTO items_fts(rowid, item_name, item_name_normalized, brand, category)
    VALUES (new.id, new.item_name, new.item_name_normalized, new.brand, new.category);
END;

-- Trigger to update FTS5 table
CREATE TRIGGER items_fts_update AFTER UPDATE ON items BEGIN
    UPDATE items_fts
    SET item_name = new.item_name,
        item_name_normalized = new.item_name_normalized,
        brand = new.brand,
        category = new.category
    WHERE rowid = new.id;
END;

-- Trigger to delete from FTS5 table
CREATE TRIGGER items_fts_delete AFTER DELETE ON items BEGIN
    DELETE FROM items_fts WHERE rowid = old.id;
END;

-- Trigger to automatically update updated_at timestamp
CREATE TRIGGER items_updated_at AFTER UPDATE ON items BEGIN
    UPDATE items SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
