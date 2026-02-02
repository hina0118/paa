-- 002_product_master: Gemini AI による商品名解析結果のキャッシュテーブル
--
-- ECサイトから抽出した商品名をAIで分解し、DBでの名寄せ精度を向上させる
-- raw_name をキーにしてキャッシュし、2回目以降はAPI呼び出しを省略

CREATE TABLE IF NOT EXISTS product_master (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- キャッシュキー: メールの原文（前後の空白除去済み）
    raw_name TEXT UNIQUE NOT NULL,
    -- 検索用正規化名（全角半角統一、小文字化、記号除去済み）
    normalized_name TEXT NOT NULL,
    -- AIが抽出した情報
    maker TEXT,                          -- メーカー名（例: KADOKAWA）
    series TEXT,                         -- シリーズ名（例: Re:ゼロから始める異世界生活）
    product_name TEXT,                   -- 純粋な商品名（例: レム 優雅美人ver.）
    scale TEXT,                          -- スケール（例: 1/7, NON）
    is_reissue INTEGER NOT NULL DEFAULT 0 CHECK(is_reissue IN (0, 1)),  -- 再販フラグ
    -- メタデータ
    platform_hint TEXT,                  -- 最初に発見されたサイト（hobbysearch, amazon等）
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- インデックス
CREATE INDEX IF NOT EXISTS idx_product_master_normalized_name ON product_master(normalized_name);
CREATE INDEX IF NOT EXISTS idx_product_master_maker ON product_master(maker) WHERE maker IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_product_master_series ON product_master(series) WHERE series IS NOT NULL;

-- 更新トリガー
CREATE TRIGGER IF NOT EXISTS product_master_updated_at AFTER UPDATE ON product_master BEGIN
    UPDATE product_master SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
