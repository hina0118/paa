-- Create order_htmls junction table for many-to-many relationship between orders and htmls
CREATE TABLE IF NOT EXISTS order_htmls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    html_id INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (html_id) REFERENCES htmls(id) ON DELETE CASCADE,
    UNIQUE (order_id, html_id)
);

-- Index for order_id lookup
CREATE INDEX idx_order_htmls_order_id ON order_htmls(order_id);

-- Index for html_id lookup
CREATE INDEX idx_order_htmls_html_id ON order_htmls(html_id);
