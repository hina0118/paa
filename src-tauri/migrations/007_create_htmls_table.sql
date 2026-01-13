-- Create htmls table for order detail page HTML storage
CREATE TABLE IF NOT EXISTS htmls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT UNIQUE NOT NULL,
    html_content TEXT,
    analysis_status TEXT NOT NULL DEFAULT 'pending' CHECK(analysis_status IN ('pending', 'completed')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Index for URL lookup
CREATE INDEX idx_htmls_url ON htmls(url);

-- Index for filtering by analysis status
CREATE INDEX idx_htmls_analysis_status ON htmls(analysis_status);
