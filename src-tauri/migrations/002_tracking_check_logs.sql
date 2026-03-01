-- 002_tracking_check_logs: 配送追跡チェック記録テーブル
-- 配送業者HPを確認した結果を保存する。1回の確認 = 1レコード（チェック記録型）。
-- 将来の自動スクレイピングでも同テーブルに INSERT するだけで済む設計。

CREATE TABLE IF NOT EXISTS tracking_check_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    delivery_id     INTEGER NOT NULL,
    checked_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- チェック自体の結果: success=取得成功 / failed=エラー / not_found=追跡番号不明
    check_status    TEXT NOT NULL DEFAULT 'success'
                    CHECK(check_status IN ('success', 'failed', 'not_found')),
    -- 確認時点の配送ステータス（deliveries.delivery_status と同じ値域、check_status='success'時のみ有効）
    delivery_status TEXT
                    CHECK(delivery_status IS NULL OR delivery_status IN (
                        'not_shipped', 'preparing', 'shipped', 'in_transit',
                        'out_for_delivery', 'delivered', 'failed', 'returned', 'cancelled'
                    )),
    -- 配送業者サイトの最新イベント説明文（例: "品川営業所に到着しました"）
    description     TEXT,
    -- 最新イベントの場所・営業所名（例: "品川営業所"）
    location        TEXT,
    -- check_status='failed' のときの理由・エラーメッセージ
    error_message   TEXT,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (delivery_id) REFERENCES deliveries(id) ON DELETE CASCADE
);

-- 配送ID + チェック日時の降順（最新チェック結果取得に使用）
CREATE INDEX IF NOT EXISTS idx_tracking_check_logs_delivery_id_checked_at
    ON tracking_check_logs(delivery_id, checked_at DESC);

-- チェック日時の降順（全件を新しい順に表示する場合に使用）
CREATE INDEX IF NOT EXISTS idx_tracking_check_logs_checked_at
    ON tracking_check_logs(checked_at DESC);
