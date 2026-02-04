# item_name_normalized リレーションキー化に伴う不要項目の分析

**作成日**: 2026-02-04

## 概要

`item_name_normalized` が images / product_master のリレーションキーとなったため、不要になった項目を洗い出した。

---

## 1. images テーブルの item_id

### 現状の役割

| 用途                  | item_name_normalized あり                 | item_name_normalized NULL |
| --------------------- | ----------------------------------------- | ------------------------- |
| リレーション（JOIN）  | 使わない（item_name_normalized で JOIN）  | **必須**（唯一のリンク）  |
| 画像保存時の検索      | item_name_normalized で検索               | item_id で検索            |
| 画像保存時の挿入/更新 | item_id を「最終更新した item」として保存 | item_id で挿入/更新       |
| ON DELETE CASCADE     | item 削除時に画像削除                     | 同上                      |

### 削除可否

- **item_name_normalized がある場合**: リレーションには不要。`orders-queries` の JOIN は `item_name_normalized` のみ使用。
- **item_name_normalized が NULL の場合**: item_id が**唯一のリンク**。削除すると画像を紐付けられない。

### 結論

| 方針                  | 対応                                                                       |
| --------------------- | -------------------------------------------------------------------------- |
| **A. item_id を削除** | ✅ 採用済み。item_name_normalized が NULL の item には画像を登録できない。 |
| **B. item_id を残す** | -                                                                          |

**実施済み**: 方針 A で images から item_id を削除した。

---

## 2. images の idx_images_item_id

- **役割**: `WHERE item_id = ?` の検索用インデックス
- **削除条件**: images.item_id を削除する場合、本インデックスも削除

---

## 3. images の FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE

- **役割**: item 削除時に画像を自動削除
- **注意**: 画像は item_name_normalized で共有されるため、item A 削除時に画像を消すと、同じ正規化名の item B の画像も消える。CASCADE の意味が変わる。
- **削除条件**: images.item_id を削除する場合、本制約も削除

---

## 4. items.brand / items.category

- **役割**: メールパース時の生データ。product_master（maker/series/scale）のフォールバックとして表示に使用
- **リレーションとの関係**: item_name_normalized のリレーションキー化とは無関係
- **結論**: **削除しない**。product_master が未解析の item の表示に必要

---

## 5. その他の確認項目

| 項目                                | 状態                                                                        |
| ----------------------------------- | --------------------------------------------------------------------------- |
| table-viewer の `item_id: '商品ID'` | images テーブル表示用ラベル。item_id 削除時は `item_name_normalized` に変更 |
| e2e-mock-db の IMAGES_SCHEMA        | item_id 削除時はスキーマから除外                                            |
| README の images 説明               | `item_id` 記載を `item_name_normalized` に更新                              |
| .serena/memories/database_schema.md | スキーマ変更に合わせて更新                                                  |

---

## 6. 削除実施時の変更一覧（方針 A 採用時）

1. **001_init.sql**: images から `item_id`、`FOREIGN KEY`、`idx_images_item_id` を削除
2. **lib.rs**: save_image_from_url から item_id 関連のロジックを削除。item_name_normalized が NULL の場合は画像保存をスキップ
3. **e2e-mock-db.ts**: IMAGES_SCHEMA から item_id を削除
4. **e2e-mock-db.test.ts**: カラム数・カラム名の期待値を更新
5. **table-viewer.tsx**: IMAGES_COLUMN_LABELS を item_name_normalized に変更
6. **README.md**: images の説明を更新
7. **docs/architecture/product-name-normalization.md**: item_id の記述を削除
8. **.serena/memories/database_schema.md**: 更新
