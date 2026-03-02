-- 001_init: 統合スキーマ
-- 注: CREATE TABLE IF NOT EXISTS のため、既存DBには適用されない。新規インストール時のみ有効。
-- sync_metadata, parse_metadata, parse_skipped, window_settings は paa_config.json で管理するため作成しない。

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
CREATE INDEX IF NOT EXISTS idx_orders_order_number_shop_domain ON orders(order_number, shop_domain);
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
    tokenize='trigram'
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
    item_name_normalized TEXT NOT NULL,
    file_name TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (item_name_normalized)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_images_item_name_normalized ON images(item_name_normalized);

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
-- tracking_check_logs
-- 配送業者HPを確認した結果を保存する。tracking_number ごとに最新 1 件のみ保持（UPSERT 設計）。
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tracking_check_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    tracking_number TEXT NOT NULL,
    checked_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- チェック自体の結果: success=取得成功 / failed=エラー / not_found=追跡番号不明
    check_status    TEXT NOT NULL DEFAULT 'success'
                    CHECK(check_status IN ('success', 'failed', 'not_found')),
    -- 確認時点の配送ステータス（deliveries.delivery_status と同じ値域）
    -- check_status='success' 時は実際の配送状況（ただし HTML 解析結果が unknown で判定不能な場合は NULL）
    -- check_status='not_found' 時は 'delivered' 扱いで保存される / check_status='failed' 時は NULL
    delivery_status TEXT
                    CHECK(delivery_status IS NULL OR delivery_status IN (
                        'not_shipped', 'preparing', 'shipped', 'in_transit',
                        'out_for_delivery', 'delivered', 'failed', 'returned', 'cancelled'
                    )),
    -- 配送業者サイトの最新イベント説明文（例: "品川営業所に到着しました"）
    description     TEXT,
    -- 最新イベントの場所・営業所名（例: "品川営業所"）
    location        TEXT,
    -- check_status='failed' のときの理由・エラーメッセージ
    error_message   TEXT,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
-- tracking_number ごとに最新 1 件のみ保持（UPSERT の衝突キーとして使用）
-- 【設計上の制約】同一追跡番号が複数配送に紐づく場合（注文分割等）は最新の結果で上書きされる。
-- 本テーブルは「追跡番号ごとの最新チェック結果」の記録を目的としており、配送単位のフル履歴保持は対象外とする。
CREATE UNIQUE INDEX IF NOT EXISTS idx_tracking_check_logs_tracking_number
    ON tracking_check_logs(tracking_number);
-- チェック日時の降順（全件を新しい順に表示する場合に使用）
CREATE INDEX IF NOT EXISTS idx_tracking_check_logs_checked_at
    ON tracking_check_logs(checked_at DESC);

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

-- デフォルト設定行は起動時に plugins::ensure_default_settings() が INSERT OR IGNORE で投入するため、
-- ここには記載しない。新規店舗の追加は src-tauri/src/plugins/registry.rs を参照。

-- -----------------------------------------------------------------------------
-- product_master (Gemini AI による商品名解析結果のキャッシュ)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS product_master (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    raw_name TEXT UNIQUE NOT NULL,
    normalized_name TEXT NOT NULL,
    maker TEXT,
    series TEXT,
    product_name TEXT,
    scale TEXT,
    is_reissue INTEGER NOT NULL DEFAULT 0 CHECK(is_reissue IN (0, 1)),
    platform_hint TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_product_master_normalized_name ON product_master(normalized_name);
CREATE INDEX IF NOT EXISTS idx_product_master_maker ON product_master(maker) WHERE maker IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_product_master_series ON product_master(series) WHERE series IS NOT NULL;
CREATE TRIGGER IF NOT EXISTS product_master_updated_at AFTER UPDATE ON product_master BEGIN
    UPDATE product_master SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- -----------------------------------------------------------------------------
-- orders: order_number のケース非依存検索用インデックス
-- -----------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_orders_order_number_shop_domain_nocase
ON orders(order_number COLLATE NOCASE, shop_domain);

-- -----------------------------------------------------------------------------
-- manual_overrides: 手動上書き・除外テーブル
-- -----------------------------------------------------------------------------
-- アイテム単位の手動修正
-- ビジネスキー: (shop_domain, order_number, original_item_name, original_brand)
CREATE TABLE IF NOT EXISTS item_overrides (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_domain TEXT NOT NULL,
    order_number TEXT NOT NULL COLLATE NOCASE,
    original_item_name TEXT NOT NULL,
    original_brand TEXT NOT NULL DEFAULT '',
    -- 上書きフィールド (NULL = 上書きなし)
    item_name TEXT,
    price INTEGER,
    quantity INTEGER,
    brand TEXT,
    category TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (shop_domain, order_number, original_item_name, original_brand)
);
CREATE TRIGGER IF NOT EXISTS item_overrides_updated_at AFTER UPDATE ON item_overrides BEGIN
    UPDATE item_overrides SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- 注文単位の手動修正
-- ビジネスキー: (shop_domain, order_number)
CREATE TABLE IF NOT EXISTS order_overrides (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_domain TEXT NOT NULL,
    order_number TEXT NOT NULL COLLATE NOCASE,
    -- 上書きフィールド (NULL = 上書きなし)
    new_order_number TEXT,
    order_date TEXT,
    shop_name TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (shop_domain, order_number)
);
CREATE TRIGGER IF NOT EXISTS order_overrides_updated_at AFTER UPDATE ON order_overrides BEGIN
    UPDATE order_overrides SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- アイテム除外リスト（論理削除: 表示クエリ側でフィルタ + 再パース時もブロック）
CREATE TABLE IF NOT EXISTS excluded_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_domain TEXT NOT NULL,
    order_number TEXT NOT NULL COLLATE NOCASE,
    item_name TEXT NOT NULL,
    brand TEXT NOT NULL DEFAULT '',
    reason TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (shop_domain, order_number, item_name, brand)
);

-- 注文除外リスト（論理削除: 表示クエリ側でフィルタ + 再パース時もブロック）
CREATE TABLE IF NOT EXISTS excluded_orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_domain TEXT NOT NULL,
    order_number TEXT NOT NULL COLLATE NOCASE,
    reason TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (shop_domain, order_number)
);
