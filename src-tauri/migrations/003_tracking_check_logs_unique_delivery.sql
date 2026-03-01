-- 003: tracking_check_logs を UPSERT 対応に変更
-- delivery_id ごとに最新 1 件のみ保持する設計に変更する。

-- 重複レコードを削除（各 delivery_id で最大 id = 最新 1 件を残す）
DELETE FROM tracking_check_logs
WHERE id NOT IN (
    SELECT MAX(id) FROM tracking_check_logs GROUP BY delivery_id
);

-- 旧複合インデックス（delivery_id, checked_at DESC）は不要になるため削除
DROP INDEX IF EXISTS idx_tracking_check_logs_delivery_id_checked_at;

-- delivery_id に UNIQUE インデックスを追加（UPSERT の衝突キーとして使用）
CREATE UNIQUE INDEX idx_tracking_check_logs_delivery_id
    ON tracking_check_logs(delivery_id);
