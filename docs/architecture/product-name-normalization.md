# 商品名正規化による items/images/product_master 連携

## 概要

items テーブルに正規化商品名（`item_name_normalized`）を登録し、images および product_master を `item_name_normalized` で関連付ける設計。

## 設計の目的

- **パース再実行時も画像を維持**: 商品名正規化をキーにすることで、パース再実行後も同じ画像を参照できる
- **同一商品の画像共有**: 同じ正規化商品名を持つ複数の item（異なる注文）が同一画像を共有する

## images テーブルの設計

- **item_name_normalized**: 画像の論理キー。同じ正規化名の item が同一画像を共有する
- **item_id**: 画像を最後に更新した item の参照（所有権ではない）。`WHERE item_id = X` で検索しても、別の item が同じ正規化名で画像を更新した場合は見つからない
- **UNIQUE(item_name_normalized)**: 非 NULL の場合は1件のみ。NULL は複数許可（partial unique index）

## product_master 連携

- `item_name_normalized` が NULL の item（正規化できない商品名）は product_master データを表示しない
- これは意図した動作。正規化できない商品は product_master 連携対象外とする

## 関連 PR

- [#62 feat: 商品名正規化によるitems/images/product_master連携](https://github.com/hina0118/paa/pull/62)
