-- 012_manual_overrides: 手動上書き・除外テーブル
-- パースで再構築される orders/items/deliveries とは独立したテーブル群。
-- clear_order_tables() の影響を受けない。

-- アイテム単位の手動修正
-- ビジネスキー: (shop_domain, order_number, original_item_name, original_brand)
-- save_order_in_tx の重複チェックと同じキー
CREATE TABLE IF NOT EXISTS item_overrides (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_domain TEXT NOT NULL,
    order_number TEXT NOT NULL,
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

CREATE INDEX IF NOT EXISTS idx_item_overrides_key
ON item_overrides(shop_domain, order_number, original_item_name, original_brand);

CREATE TRIGGER IF NOT EXISTS item_overrides_updated_at AFTER UPDATE ON item_overrides BEGIN
    UPDATE item_overrides SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- 注文単位の手動修正
-- ビジネスキー: (shop_domain, order_number)
CREATE TABLE IF NOT EXISTS order_overrides (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_domain TEXT NOT NULL,
    order_number TEXT NOT NULL,
    -- 上書きフィールド (NULL = 上書きなし)
    new_order_number TEXT,
    order_date TEXT,
    shop_name TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (shop_domain, order_number)
);

CREATE INDEX IF NOT EXISTS idx_order_overrides_key
ON order_overrides(shop_domain, order_number);

CREATE TRIGGER IF NOT EXISTS order_overrides_updated_at AFTER UPDATE ON order_overrides BEGIN
    UPDATE order_overrides SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- アイテム除外リスト（物理削除 + 再パース時もブロック）
CREATE TABLE IF NOT EXISTS excluded_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_domain TEXT NOT NULL,
    order_number TEXT NOT NULL,
    item_name TEXT NOT NULL,
    brand TEXT NOT NULL DEFAULT '',
    reason TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (shop_domain, order_number, item_name, brand)
);

CREATE INDEX IF NOT EXISTS idx_excluded_items_key
ON excluded_items(shop_domain, order_number, item_name, brand);

-- 注文除外リスト（物理削除 + 再パース時もブロック）
CREATE TABLE IF NOT EXISTS excluded_orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_domain TEXT NOT NULL,
    order_number TEXT NOT NULL,
    reason TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (shop_domain, order_number)
);

CREATE INDEX IF NOT EXISTS idx_excluded_orders_key
ON excluded_orders(shop_domain, order_number);

