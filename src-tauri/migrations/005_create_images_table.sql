-- Create images table for product image storage
CREATE TABLE IF NOT EXISTS images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id INTEGER NOT NULL,
    image_data BLOB,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE,
    UNIQUE (item_id)
);

-- Unique index for item_id (1 product = 1 image constraint)
CREATE UNIQUE INDEX idx_images_item_id ON images(item_id);
