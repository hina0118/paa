# Issue #244: ダッシュボード画面の更新

## 対応内容

### 1. 表示項目の整理

- **注文数カードを削除**: パース済み注文数は非表示に
- **EmailStatsSection を削除**: メール統計・同期状況・パース状況・平均本文長・データ品質のカードを削除
- **配送状況を絞り込み**: 配達済み・発送済み・配送中・配達中・未発送の5項目のみ表示（準備中・配送失敗・返品・キャンセルは非表示）

### 2. 商品名解析・店舗設定・商品画像の表示改善

- **1行表示**: 3つのカードを `grid md:grid-cols-3` で横並びに配置
- **商品名解析**: 解析済み/全体数と網羅率を表示（`items_with_parsed` / `distinct_items_with_normalized`）
- **店舗設定**: 有効/登録済みを「26 / 26（有効 / 登録済み）」形式で表示
- **商品画像**: キャッシュ件数/全体数と網羅率を表示（`images_count` / `distinct_items_with_normalized`）
- **説明文を復元**: 各カードに CardDescription を追加

### 3. 1年以上未発送の件数表示

- **DeliveryStats** に `not_shipped_over_1_year` を追加
- 注文日または作成日が1年以上前で、最新ステータスが未発送の注文件数を表示
- 件数が1件以上の場合はアンバー色で強調表示

### 4. 商品数の指標統一

- **商品数** を `total_items`（items 行数）から `distinct_items_with_normalized`（正規化名のユニーク数）に変更
- 商品名解析・商品画像と同一指標で表示し、数値の整合性を確保
- 説明文を「ユニーク商品（正規化名）」に変更

### 5. カラーバーの統一

- 全カードに上部グラデーションバーを追加
- 3行目（商品名解析・店舗設定・商品画像）は `from-violet-500 to-emerald-500` で統一
- 1行目は商品数 `violet→emerald`、合計金額 `emerald→cyan`
- 2行目（配送状況）は `violet→emerald`

### 6. バックエンド変更

- **OrderStats**: `distinct_items_with_normalized` を追加
- **DeliveryStats**: `not_shipped_over_1_year` を追加
- **MiscStats**: `distinct_items_with_normalized` を追加（商品画像の網羅率計算用）

### 7. 削除・整理

- `EmailStatsSection` コンポーネントを削除
- `useParse` / `useSync` のダッシュボードからの参照を削除
- 関連テストの更新・整理
