-- 設定を paa_config.json に切り出したため、sync_metadata / parse_metadata から設定カラムを削除
-- batch_size, max_iterations は設定ファイルで管理する

-- sync_metadata: batch_size, max_iterations を削除
CREATE TABLE sync_metadata_new (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    oldest_fetched_date TEXT,
    sync_status TEXT NOT NULL DEFAULT 'idle' CHECK(sync_status IN ('idle', 'syncing', 'paused', 'error')),
    total_synced_count INTEGER NOT NULL DEFAULT 0,
    last_sync_started_at TEXT,
    last_sync_completed_at TEXT,
    last_error_message TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);
INSERT INTO sync_metadata_new (id, oldest_fetched_date, sync_status, total_synced_count, last_sync_started_at, last_sync_completed_at, last_error_message, created_at, updated_at)
SELECT id, oldest_fetched_date, sync_status, total_synced_count, last_sync_started_at, last_sync_completed_at, last_error_message, created_at, updated_at
FROM sync_metadata;
DROP TABLE sync_metadata;
ALTER TABLE sync_metadata_new RENAME TO sync_metadata;
CREATE TRIGGER update_sync_metadata_timestamp
    AFTER UPDATE ON sync_metadata
    FOR EACH ROW
BEGIN
    UPDATE sync_metadata SET updated_at = datetime('now') WHERE id = 1;
END;

-- parse_metadata: batch_size を削除
CREATE TABLE parse_metadata_new (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    parse_status TEXT NOT NULL DEFAULT 'idle' CHECK(parse_status IN ('idle', 'running', 'completed', 'error')),
    last_parse_started_at DATETIME,
    last_parse_completed_at DATETIME,
    total_parsed_count INTEGER NOT NULL DEFAULT 0,
    last_error_message TEXT
);
INSERT INTO parse_metadata_new (id, parse_status, last_parse_started_at, last_parse_completed_at, total_parsed_count, last_error_message)
SELECT id, parse_status, last_parse_started_at, last_parse_completed_at, total_parsed_count, last_error_message
FROM parse_metadata;
DROP TABLE parse_metadata;
ALTER TABLE parse_metadata_new RENAME TO parse_metadata;
