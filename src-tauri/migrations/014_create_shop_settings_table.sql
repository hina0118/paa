-- Create shop_settings table for managing email sender addresses and parser routing
-- Note: UNIQUE constraint on sender_address removed to allow multiple parsers per address
CREATE TABLE IF NOT EXISTS shop_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_name TEXT NOT NULL,
    sender_address TEXT NOT NULL,
    parser_type TEXT NOT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 1 CHECK(is_enabled IN (0, 1)),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Index for efficient lookups by sender address
CREATE INDEX idx_shop_settings_sender_address ON shop_settings(sender_address);

-- Index for filtering enabled shops
CREATE INDEX idx_shop_settings_is_enabled ON shop_settings(is_enabled);

-- Note: subject_filter will be added by migration 015
