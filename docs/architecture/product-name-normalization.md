# 商品名正規化による items/images/product_master 連携

## 概要

items テーブルに正規化商品名（`item_name_normalized`）を登録し、images および product_master を `item_name_normalized` で関連付ける設計。

## 設計の目的

- **パース再実行時も画像を維持**: 商品名正規化をキーにすることで、パース再実行後も同じ画像を参照できる
- **同一商品の画像共有**: 同じ正規化商品名を持つ複数の item（異なる注文）が同一画像を共有する

## images テーブルの設計

- **item_name_normalized**: リレーションキー。同じ正規化名の item が同一画像を共有する
- **UNIQUE(item_name_normalized)**: 1件のみ
- 正規化できない商品名（NULL）の item には画像を登録できない

## product_master 連携

- `item_name_normalized` が NULL の item（正規化できない商品名）は product_master データを表示しない
- これは意図した動作。正規化できない商品は product_master 連携対象外とする

## スキーマ

`items`／`images` のスキーマは `001_init.sql` に集約されている。新規インストール時は 001 で作成される。

## 既存DBのマイグレーション方針

本設計では、`items`／`images` に対して以下のスキーマ変更が発生する。

- `items.item_name_normalized` カラムの追加
- `images.item_name_normalized` カラムおよび `UNIQUE (item_name_normalized) WHERE item_name_normalized IS NOT NULL` の partial unique index
- `images` の `UNIQUE(item_id)` から `UNIQUE(item_name_normalized)` への制約変更

これらは `001_init.sql` の `CREATE TABLE IF NOT EXISTS` では既存テーブルに適用されないため、既存の本番／検証環境には **別途マイグレーションを適用する必要がある**。

推奨対応:

1. 本設計用の DDL を含むマイグレーションファイルを新規追加する（例: `003_images_item_name_normalized.sql`）
2. アプリケーションをデプロイする前に、対象環境へマイグレーションを適用する
3. スキーマ変更後、必要に応じて `items.item_name_normalized` を既存データに対してバックフィルする
4. 正常にバックフィルできたことを確認してから、本設計のコードを有効化する

なお、新規インストールのみを対象とする場合は、001 のスキーマでそのまま利用可能である。

## 関連 PR

- [#62 feat: 商品名正規化によるitems/images/product_master連携](https://github.com/hina0118/paa/pull/62)
