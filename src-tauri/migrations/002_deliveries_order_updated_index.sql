-- 002: deliveries の order_id + updated_at 複合インデックス
-- loadOrderItems の latest_delivery CTE（ORDER BY updated_at DESC）を効率化
CREATE INDEX IF NOT EXISTS idx_deliveries_order_id_updated_at ON deliveries(order_id, updated_at DESC);
