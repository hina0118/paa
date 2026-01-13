-- Create order_emails junction table for many-to-many relationship between orders and emails
CREATE TABLE IF NOT EXISTS order_emails (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    email_id INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE,
    UNIQUE (order_id, email_id)
);

-- Index for order_id lookup
CREATE INDEX idx_order_emails_order_id ON order_emails(order_id);

-- Index for email_id lookup
CREATE INDEX idx_order_emails_email_id ON order_emails(email_id);
