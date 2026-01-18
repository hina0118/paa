-- Create window_settings table to store window position and size
CREATE TABLE IF NOT EXISTS window_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- Single-row table
    width INTEGER NOT NULL DEFAULT 800,
    height INTEGER NOT NULL DEFAULT 600,
    x INTEGER,
    y INTEGER,
    maximized INTEGER NOT NULL DEFAULT 0, -- 0 = false, 1 = true
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Initialize with single row (idempotent: does nothing if row already exists)
INSERT OR IGNORE INTO window_settings (id, width, height) VALUES (1, 800, 600);

-- Trigger to update updated_at on changes
CREATE TRIGGER update_window_settings_timestamp
    AFTER UPDATE ON window_settings
    FOR EACH ROW
BEGIN
    UPDATE window_settings SET updated_at = datetime('now') WHERE id = 1;
END;
