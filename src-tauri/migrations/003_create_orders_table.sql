-- Create orders table for order management
CREATE TABLE IF NOT EXISTS orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    shop_domain TEXT,
    order_number TEXT,
    order_date DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for shop_domain filtering
CREATE INDEX idx_orders_shop_domain ON orders(shop_domain);

-- Index for order_date sorting
CREATE INDEX idx_orders_order_date ON orders(order_date DESC);

-- Index for created_at sorting
CREATE INDEX idx_orders_created_at ON orders(created_at DESC);

-- Trigger to automatically update updated_at timestamp
CREATE TRIGGER orders_updated_at AFTER UPDATE ON orders BEGIN
    UPDATE orders SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
