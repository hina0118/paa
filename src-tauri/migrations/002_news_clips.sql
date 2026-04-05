-- ニュースクリップ
-- ユーザーが保存した記事を格納する。AIによる要約・タグを保持
CREATE TABLE IF NOT EXISTS news_clips (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT    NOT NULL,
    url         TEXT    NOT NULL,
    source_name TEXT    NOT NULL,
    published_at TEXT,
    summary     TEXT,
    tags        TEXT    NOT NULL DEFAULT '[]', -- JSON 配列
    clipped_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (url)
);
