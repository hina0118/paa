-- 011: order_number のケース非依存検索用インデックス
-- LOWER(order_number) の代わりに COLLATE NOCASE を使用し、インデックスを有効活用する。
-- 既存の idx_orders_order_number_shop_domain は BINARY 照合のため、
-- order_number COLLATE NOCASE 用の別インデックスを追加する。

CREATE INDEX IF NOT EXISTS idx_orders_order_number_shop_domain_nocase
ON orders(order_number COLLATE NOCASE, shop_domain);
