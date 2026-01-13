-- Create deliveries table for delivery tracking
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

-- Index for order_id lookup
CREATE INDEX idx_deliveries_order_id ON deliveries(order_id);

-- Index for tracking_number lookup
CREATE INDEX idx_deliveries_tracking_number ON deliveries(tracking_number) WHERE tracking_number IS NOT NULL;

-- Index for delivery_status filtering
CREATE INDEX idx_deliveries_status ON deliveries(delivery_status);

-- Index for delivery_status and updated_at sorting
CREATE INDEX idx_deliveries_status_updated ON deliveries(delivery_status, updated_at DESC);

-- Trigger to automatically update updated_at timestamp
CREATE TRIGGER deliveries_updated_at AFTER UPDATE ON deliveries BEGIN
    UPDATE deliveries SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
