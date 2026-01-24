-- Create parse_metadata table for tracking parse operations
CREATE TABLE IF NOT EXISTS parse_metadata (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    parse_status TEXT NOT NULL DEFAULT 'idle' CHECK(parse_status IN ('idle', 'running', 'completed', 'error')),
    last_parse_started_at DATETIME,
    last_parse_completed_at DATETIME,
    total_parsed_count INTEGER NOT NULL DEFAULT 0,
    last_error_message TEXT
);

-- Insert default row
INSERT OR IGNORE INTO parse_metadata (id, parse_status) VALUES (1, 'idle');
