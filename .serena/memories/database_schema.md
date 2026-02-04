# データベーススキーマ (予定)

このプロジェクトは、10年以上の購入履歴を管理するためにSQLiteデータベースを使用する予定です。

## 主要テーブル構造

### `orders` (注文単位)

注文全体の情報を保存するテーブル。

| カラム名           | 型                  | 説明                                  |
| ------------------ | ------------------- | ------------------------------------- |
| `id`               | INTEGER PRIMARY KEY | 注文ID (自動採番)                     |
| `gmail_message_id` | TEXT                | Gmail APIから取得したメッセージID     |
| `shop_domain`      | TEXT                | ECサイトのドメイン (例: amazon.co.jp) |
| `raw_body`         | TEXT                | メール本文の生データ                  |
| `raw_html`         | TEXT                | 注文詳細ページのHTML生データ          |
| `order_date`       | DATETIME            | 注文日時                              |

### `items` (商品単位)

1つの注文内の個別商品を管理するテーブル。

| カラム名          | 型                  | 説明                                      |
| ----------------- | ------------------- | ----------------------------------------- |
| `id`              | INTEGER PRIMARY KEY | 商品ID (自動採番)                         |
| `order_id`        | INTEGER             | 注文ID (FK: orders.id)                    |
| `item_name`       | TEXT                | 商品名                                    |
| `price`           | INTEGER             | 価格 (円)                                 |
| `tracking_number` | TEXT                | 配送追跡番号                              |
| `delivery_status` | TEXT                | 配送ステータス (例: "配送中", "配達完了") |

### `images` (画像データ)

商品画像を保存するテーブル。`item_name_normalized` で items と関連付け。

| カラム名               | 型                  | 説明                                     |
| ---------------------- | ------------------- | ---------------------------------------- |
| `id`                   | INTEGER PRIMARY KEY | 画像ID (自動採番)                        |
| `item_name_normalized` | TEXT                | 正規化商品名 (リレーションキー)          |
| `file_name`            | TEXT                | 画像ファイル名 (app_data_dir/images/ 内) |
| `created_at`           | DATETIME            | 作成日時                                 |

## データフロー

1. **Gmail同期**: `orders` テーブルに `raw_body` と `gmail_message_id` を保存
2. **HTML取得**: 対応する `orders` レコードに `raw_html` を追加
3. **パース**: `raw_html` から商品情報を抽出し、`items` テーブルに保存
4. **画像取得**: 商品名をもとに画像APIから取得し、`images` テーブルに保存
5. **配送追跡**: `items.tracking_number` を使って定期的にステータスを更新

## インデックス (最適化)

- `orders.gmail_message_id` にユニークインデックス
- `orders.shop_domain` にインデックス
- `items.order_id` に外部キーインデックス
- `items.item_name` に全文検索インデックス (FTS5)
- `images.item_name_normalized` にユニークインデックス

## 注意事項

- すべてのデータはローカルに保存 (オフライン動作)
- 画像データはBLOB形式で直接DBに保存
- 曖昧検索のためにFuzzy Search機能を実装予定
