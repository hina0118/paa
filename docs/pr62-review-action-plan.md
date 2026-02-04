# PR #62 レビューコメント対応計画

**PR**: [#62 feat: 商品名正規化によるitems/images/product_master連携](https://github.com/hina0118/paa/pull/62)  
**作成日**: 2026-02-04  
**更新日**: 2026-02-04  
**未対応コメント数**: **0件**（全17件対応済み）

---

## 概要

PR 62 に対する GitHub Copilot のレビューコメントを整理し、対応計画を作成しました。  
指摘と照らし合わせてコード・ドキュメントを更新済み。

### PR説明の更新案（GitHub で手動更新）

指摘「003マイグレーションが存在しない」への対応として、PR説明の「変更内容」を以下に修正することを推奨:

```
### バックエンド
- `repository.rs`: 商品INSERT時に`normalize_product_name`で正規化名を設定
- `lib.rs`: 画像保存時に`item_name_normalized`を取得・保存
- `001_init.sql`: images に `item_name_normalized` カラム追加、UNIQUE 制約を `item_name_normalized` に変更

※ 001 は CREATE TABLE IF NOT EXISTS のため、既存DBには適用されない。新規インストール時のみ有効。
既存DBへの対応が必要な場合は、別途マイグレーション（例: 003）を追加すること。
```

---

## P0: Critical（必須対応）— 2件

| #   | ファイル                            | 行  | 指摘内容                                                                                                                                                    | 対応方針                                           |
| --- | ----------------------------------- | --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| 1   | `src-tauri/migrations/001_init.sql` | 110 | **003マイグレーションが存在しない** — PR説明は「003マイグレーション: imagesにitem_name_normalizedカラム追加」と記載しているが、実際には001_init.sqlのみ変更 | ✅ 001 に集約（スキーマを 001 に統合、003 は削除） |
| 2   | `src-tauri/migrations/001_init.sql` | 110 | **既存DBでUNIQUE制約の競合** — 既存DBには `UNIQUE(item_id)` が残ったまま                                                                                    | ✅ 001 に集約                                      |

---

## P1: Important（推奨対応）— 1件

| #   | ファイル               | 行   | 指摘内容                                               | 対応方針                                                               |
| --- | ---------------------- | ---- | ------------------------------------------------------ | ---------------------------------------------------------------------- |
| 3   | `src-tauri/src/lib.rs` | 1340 | **既存DBでitem_name_normalizedカラムがない場合に失敗** | ✅ 001 に集約（新規インストール時は 001 で正しいスキーマが作成される） |

---

## P2: Nitpick（任意対応）— 4件

| #   | ファイル                                          | 行  | 指摘内容                                                                                                            | 対応方針                                                         |
| --- | ------------------------------------------------- | --- | ------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| 4   | `src/components/orders/order-item-card.tsx`       | 74  | **order-item-row.tsx と重複する条件分岐ロジック** — maker/series/scale と brand/category のフォールバック表示が重複 | ✅ getProductMetadata を utils.ts に抽出、両コンポーネントで使用 |
| 5   | `src/components/orders/order-item-row.tsx`        | 72  | **order-item-card.tsx と重複** — 上記と同一                                                                         | ✅ 同上                                                          |
| 6   | `src/lib/e2e-mock-db.test.ts`                     | 23  | **コメントが不明瞭** — 「5カラム」とあるが、`item_name_normalized` が追加された旨を明示すべき                       | ✅ コメント追記済み                                              |
| 7   | `docs/architecture/product-name-normalization.md` | 22  | **既存DBのマイグレーション方針が未記載**                                                                            | ✅ 「既存DBのマイグレーション方針」セクション追加済み            |

---

## 対応順序の推奨

### Phase 1: マイグレーション（P0）— 最優先

1. ~~003_images_item_name_normalized.sql~~ → **001 に集約**（スキーマを 001 に統合、003 は削除済み）

### Phase 2: コード重複解消（P2）

4. **getProductMetadata ユーティリティ** — `order-item-card.tsx` と `order-item-row.tsx` で共通化
5. **e2e-mock-db.test.ts** — コメント更新
6. **product-name-normalization.md** — 既存DBマイグレーション方針セクション追加

---

## レビューコメント一覧（未対応 6件）

| #   | ファイル                        | 行   | 優先度 | 指摘内容                                              | 状態        |
| --- | ------------------------------- | ---- | ------ | ----------------------------------------------------- | ----------- |
| 1   | `001_init.sql`                  | 110  | P0     | 003マイグレーションが存在しない、既存DBに適用されない | ✅ 対応済み |
| 2   | `001_init.sql`                  | 110  | P0     | UNIQUE制約変更が既存DBに反映されない、競合の可能性    | ✅ 対応済み |
| 3   | `lib.rs`                        | 1340 | P1     | 既存DBで item_name_normalized カラムがない場合に失敗  | ✅ 対応済み |
| 4   | `order-item-card.tsx`           | 74   | P2     | 条件分岐ロジックの重複                                | ✅ 対応済み |
| 5   | `order-item-row.tsx`            | 72   | P2     | 同上                                                  | ✅ 対応済み |
| 6   | `e2e-mock-db.test.ts`           | 23   | P2     | コメントの明確化                                      | ✅ 対応済み |
| 7   | `product-name-normalization.md` | 22   | P2     | 既存DBマイグレーション方針の追記                      | ✅ 対応済み |

---

## 対応済みコメント（11件・Resolved）

以下のコメントは GitHub 上で Resolved 済みです。

- マイグレーションの UNIQUE(item_id) と item_name_normalized の競合（初回レビュー・Outdated）
- lib.rs の画像保存ロジックのデータ整合性（初回レビュー）
- orders-queries.ts の同一商品名での画像共有（意図した動作として確認済み）
- 既存 items の item_name_normalized が NULL の場合（意図した動作・product_master 連携対象外）
- 003 の WHERE item_name_normalized IS NULL の冗長性（Outdated）
- repository.rs の空文字列→NULL 変換（既に実装済み）
- product_master JOIN の NULL 扱い（意図した動作）
- item_id の「最終更新」 semantics（設計ドキュメントで明記済み）
- pr62-review-action-plan のリポジトリコミット（本ドキュメント。アーキテクチャ doc と併用）

---

## 技術メモ

### 001 への集約

スキーマは `001_init.sql` に集約済み。003 マイグレーションは削除した。

---

## 参考リンク

- [PR #62 レビューコメント](https://github.com/hina0118/paa/pull/62)
- [PR #59 レビュー対応計画](./pr59-review-action-plan.md)（フォーマット参考）
- [商品名正規化アーキテクチャ](./architecture/product-name-normalization.md)
