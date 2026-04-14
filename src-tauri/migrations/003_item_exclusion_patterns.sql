-- 除外キーワードパターン
-- メール解析時・商品名パース時に一致した商品をスキップするためのルール
CREATE TABLE IF NOT EXISTS item_exclusion_patterns (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_domain TEXT,           -- NULL = 全ショップ / 'amazon.co.jp' など個別指定
    keyword     TEXT    NOT NULL,
    match_type  TEXT    NOT NULL DEFAULT 'contains'
                        CHECK(match_type IN ('contains', 'starts_with', 'exact')),
    note        TEXT,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_item_exclusion_patterns_shop
ON item_exclusion_patterns(shop_domain);
