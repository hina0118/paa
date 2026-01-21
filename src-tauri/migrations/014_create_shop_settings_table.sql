-- Create shop_settings table for managing email sender addresses and parser routing
CREATE TABLE IF NOT EXISTS shop_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_name TEXT NOT NULL,
    sender_address TEXT NOT NULL UNIQUE,
    parser_type TEXT NOT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 1 CHECK(is_enabled IN (0, 1)),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Index for efficient lookups by sender address
CREATE INDEX idx_shop_settings_sender_address ON shop_settings(sender_address);

-- Index for filtering enabled shops
CREATE INDEX idx_shop_settings_is_enabled ON shop_settings(is_enabled);

-- Insert default shop settings for existing known parsers
INSERT INTO shop_settings (shop_name, sender_address, parser_type, is_enabled) VALUES
    ('Amazon発送通知', 'ship-confirm@amazon.co.jp', 'amazon', 1),
    ('楽天市場', 'order@rakuten.co.jp', 'rakuten', 1),
    ('Yahoo!ショッピング', 'shopping-order-master@mail.yahoo.co.jp', 'yahoo', 1);
