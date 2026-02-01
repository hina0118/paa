-- 001_init: 統合スキーマ (旧 001-006 をすべて反映)

-- -----------------------------------------------------------------------------
-- emails
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS emails (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id TEXT UNIQUE NOT NULL,
    body_plain TEXT,
    body_html TEXT,
    analysis_status TEXT NOT NULL DEFAULT 'pending' CHECK(analysis_status IN ('pending', 'completed')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    internal_date INTEGER,
    from_address TEXT,
    subject TEXT
);
CREATE INDEX IF NOT EXISTS idx_emails_message_id ON emails(message_id);
CREATE INDEX IF NOT EXISTS idx_emails_analysis_status ON emails(analysis_status);
CREATE INDEX IF NOT EXISTS idx_emails_internal_date ON emails(internal_date);
CREATE INDEX IF NOT EXISTS idx_emails_from_address ON emails(from_address);
CREATE INDEX IF NOT EXISTS idx_emails_subject ON emails(subject);
-- get_unparsed_emails クエリのパフォーマンス改善（body_plain, from_address が NULL でない行に限定）
CREATE INDEX IF NOT EXISTS idx_emails_unparsed_filter ON emails(internal_date)
WHERE body_plain IS NOT NULL AND from_address IS NOT NULL;

-- -----------------------------------------------------------------------------
-- orders
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_domain TEXT,
    shop_name TEXT,
    order_number TEXT,
    order_date DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_orders_shop_domain ON orders(shop_domain);
CREATE INDEX IF NOT EXISTS idx_orders_order_date ON orders(order_date DESC);
CREATE INDEX IF NOT EXISTS idx_orders_created_at ON orders(created_at DESC);
CREATE TRIGGER orders_updated_at AFTER UPDATE ON orders BEGIN
    UPDATE orders SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- -----------------------------------------------------------------------------
-- items
-- -----------------------------------------------------------------------------
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
CREATE INDEX IF NOT EXISTS idx_items_order_id ON items(order_id);
CREATE INDEX IF NOT EXISTS idx_items_item_name ON items(item_name);
CREATE INDEX IF NOT EXISTS idx_items_created_at ON items(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_items_category ON items(category) WHERE category IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_items_brand ON items(brand) WHERE brand IS NOT NULL;
CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
    item_name,
    item_name_normalized,
    brand,
    category,
    content=items,
    content_rowid=id,
    tokenize='unicode61 remove_diacritics 2'
);
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
CREATE TRIGGER items_updated_at AFTER UPDATE ON items BEGIN
    UPDATE items SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- -----------------------------------------------------------------------------
-- images (file_name のみ、app_data_dir/images/ に実体保存)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id INTEGER NOT NULL,
    file_name TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE,
    UNIQUE (item_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_images_item_id ON images(item_id);

-- -----------------------------------------------------------------------------
-- deliveries
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS deliveries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    tracking_number TEXT,
    carrier TEXT,
    delivery_status TEXT NOT NULL DEFAULT 'not_shipped' CHECK(delivery_status IN ('not_shipped', 'preparing', 'shipped', 'in_transit', 'out_for_delivery', 'delivered', 'failed', 'returned', 'cancelled')),
    estimated_delivery DATETIME,
    actual_delivery DATETIME,
    last_checked_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_deliveries_order_id_updated_at ON deliveries(order_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_deliveries_tracking_number ON deliveries(tracking_number) WHERE tracking_number IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_deliveries_status ON deliveries(delivery_status);
CREATE INDEX IF NOT EXISTS idx_deliveries_status_updated ON deliveries(delivery_status, updated_at DESC);
CREATE TRIGGER deliveries_updated_at AFTER UPDATE ON deliveries BEGIN
    UPDATE deliveries SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- -----------------------------------------------------------------------------
-- htmls
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS htmls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT UNIQUE NOT NULL,
    html_content TEXT,
    analysis_status TEXT NOT NULL DEFAULT 'pending' CHECK(analysis_status IN ('pending', 'completed')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_htmls_url ON htmls(url);
CREATE INDEX IF NOT EXISTS idx_htmls_analysis_status ON htmls(analysis_status);

-- -----------------------------------------------------------------------------
-- order_emails
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS order_emails (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    email_id INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE,
    UNIQUE (order_id, email_id)
);
CREATE INDEX IF NOT EXISTS idx_order_emails_order_id ON order_emails(order_id);
CREATE INDEX IF NOT EXISTS idx_order_emails_email_id ON order_emails(email_id);

-- -----------------------------------------------------------------------------
-- order_htmls
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS order_htmls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    html_id INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (html_id) REFERENCES htmls(id) ON DELETE CASCADE,
    UNIQUE (order_id, html_id)
);
CREATE INDEX IF NOT EXISTS idx_order_htmls_order_id ON order_htmls(order_id);
CREATE INDEX IF NOT EXISTS idx_order_htmls_html_id ON order_htmls(html_id);

-- -----------------------------------------------------------------------------
-- parse_skipped（パース失敗メールの記録、無限ループ防止）
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS parse_skipped (
    email_id INTEGER PRIMARY KEY,
    error_message TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE
);

-- -----------------------------------------------------------------------------
-- sync_metadata
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS sync_metadata (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    oldest_fetched_date TEXT,
    sync_status TEXT NOT NULL DEFAULT 'idle' CHECK(sync_status IN ('idle', 'syncing', 'paused', 'error')),
    total_synced_count INTEGER NOT NULL DEFAULT 0,
    batch_size INTEGER NOT NULL DEFAULT 50,
    max_iterations INTEGER NOT NULL DEFAULT 1000,
    last_sync_started_at TEXT,
    last_sync_completed_at TEXT,
    last_error_message TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);
INSERT INTO sync_metadata (id, sync_status) VALUES (1, 'idle');
CREATE TRIGGER update_sync_metadata_timestamp
    AFTER UPDATE ON sync_metadata
    FOR EACH ROW
BEGIN
    UPDATE sync_metadata SET updated_at = datetime('now') WHERE id = 1;
END;

-- -----------------------------------------------------------------------------
-- window_settings
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS window_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    width INTEGER NOT NULL DEFAULT 800,
    height INTEGER NOT NULL DEFAULT 600,
    x INTEGER,
    y INTEGER,
    maximized INTEGER NOT NULL DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);
INSERT OR IGNORE INTO window_settings (id, width, height) VALUES (1, 800, 600);
CREATE TRIGGER update_window_settings_timestamp
    AFTER UPDATE ON window_settings
    FOR EACH ROW
BEGIN
    UPDATE window_settings SET updated_at = datetime('now') WHERE id = 1;
END;

-- -----------------------------------------------------------------------------
-- shop_settings
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS shop_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_name TEXT NOT NULL,
    sender_address TEXT NOT NULL,
    parser_type TEXT NOT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 1 CHECK(is_enabled IN (0, 1)),
    subject_filters TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (sender_address, parser_type)
);
CREATE INDEX IF NOT EXISTS idx_shop_settings_sender_address ON shop_settings(sender_address);
CREATE INDEX IF NOT EXISTS idx_shop_settings_is_enabled ON shop_settings(is_enabled);

-- 同一送信元・件名に複数パーサーを登録する場合: change/change_yoyaku, confirm/confirm_yoyaku は
-- 本文構造が異なり（[ご購入内容] vs [ご予約内容]）、試行順序は shop_name, id で一意に決まる
-- INSERT OR IGNORE でマイグレーションの冪等性を確保（二重実行時に重複キーエラーを回避）
INSERT OR IGNORE INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled) VALUES
    ('ホビーサーチ', 'hs-support@1999.co.jp', 'hobbysearch_send', '["【ホビーサーチ】ご注文の発送が完了しました"]', 1),
    ('ホビーサーチ', 'hs-support@1999.co.jp', 'hobbysearch_change', '["【ホビーサーチ】ご注文が組み替えられました"]', 1),
    ('ホビーサーチ', 'hs-support@1999.co.jp', 'hobbysearch_change_yoyaku', '["【ホビーサーチ】ご注文が組み替えられました"]', 1),
    ('ホビーサーチ', 'hs-order@1999.co.jp', 'hobbysearch_change', '["【ホビーサーチ】ご注文が組み替えられました"]', 1),
    ('ホビーサーチ', 'hs-order@1999.co.jp', 'hobbysearch_change_yoyaku', '["【ホビーサーチ】ご注文が組み替えられました"]', 1),
    ('ホビーサーチ', 'hs-order@1999.co.jp', 'hobbysearch_confirm_yoyaku', '["【ホビーサーチ】注文確認メール"]', 1),
    ('ホビーサーチ', 'hs-order@1999.co.jp', 'hobbysearch_confirm', '["【ホビーサーチ】注文確認メール"]', 1);

-- -----------------------------------------------------------------------------
-- parse_metadata
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS parse_metadata (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    parse_status TEXT NOT NULL DEFAULT 'idle' CHECK(parse_status IN ('idle', 'running', 'completed', 'error')),
    last_parse_started_at DATETIME,
    last_parse_completed_at DATETIME,
    total_parsed_count INTEGER NOT NULL DEFAULT 0,
    last_error_message TEXT,
    batch_size INTEGER NOT NULL DEFAULT 100
);
INSERT OR IGNORE INTO parse_metadata (id, parse_status) VALUES (1, 'idle');
