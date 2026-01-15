-- Create sync_metadata table to track synchronization state
CREATE TABLE IF NOT EXISTS sync_metadata (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- Single-row table
    oldest_fetched_date TEXT, -- ISO 8601 format: YYYY-MM-DDTHH:MM:SS.sssZ
    sync_status TEXT NOT NULL DEFAULT 'idle' CHECK(sync_status IN ('idle', 'syncing', 'paused', 'error')),
    total_synced_count INTEGER NOT NULL DEFAULT 0,
    batch_size INTEGER NOT NULL DEFAULT 50,
    last_sync_started_at TEXT,
    last_sync_completed_at TEXT,
    last_error_message TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Initialize with single row
INSERT INTO sync_metadata (id, sync_status) VALUES (1, 'idle');

-- Trigger to update updated_at on changes
CREATE TRIGGER update_sync_metadata_timestamp
    AFTER UPDATE ON sync_metadata
    FOR EACH ROW
BEGIN
    UPDATE sync_metadata SET updated_at = datetime('now') WHERE id = 1;
END;
