# PR #79 対応計画

**作成日**: 2026-02-08  
**PR**: [#79 feat: 組み換えメール処理の改善 - 元注文商品の自動キャンセル処理 (#27)](https://github.com/hina0118/paa/pull/79)  
**ブランチ**: `issue-27` → `main`  
**未解決レビュー**: 5件（P1: 4件、P2: 1件）

---

## PR 概要

**タイトル**: feat: 組み換えメール処理の改善 - 元注文商品の自動キャンセル処理 (#27)  
**ブランチ**: `issue-27` → `main`  
**状態**: Open  
**CI**: pending（コミット `5822371`）  
**mergeable_state**: unstable

**変更内容**:

- Issue #27 対応。組み換えメール（hobbysearch_change / hobbysearch_change_yoyaku）パース時に、元注文の商品を自動的に削除する処理を実装。
- **OrderRepository**: `apply_change_items` メソッドを追加
- **email_parse_task.rs**: hobbysearch_change / hobbysearch_change_yoyaku の場合、`save_order` の前に `apply_change_items` を呼び出し
- **parsers/mod.rs**: batch_parse_emails で同様に `apply_change_items` を呼び出し
- **テスト**: 統合テスト 3 件追加（商品削除・発送済み除外・マッチ無し）

**変更ファイル**:

- `docs/issue-27-plan.md`（新規）
- `src-tauri/src/parsers/email_parse_task.rs`
- `src-tauri/src/parsers/mod.rs`
- `src-tauri/src/repository.rs`

---

## レビューコメント一覧

### 未対応（現行ブランチへの指摘）

| #   | 優先度 | 指摘                              | 行   | 概要                                                                                                                                                                                            |
| --- | ------ | --------------------------------- | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | P1     | change_email_internal_date 未使用 | 902  | 引数が `_change_email_internal_date` で未使用。internal_date が NULL / 並び替え崩れ / 再パース時に future orders を誤削除するリスク。`internal_date` で「組み換えメールより前の注文のみ」に絞る |
| 2   | P1     | マッチ無し時の warn ログ欠如      | 990  | order_loop で1件もマッチしなかった場合に、order_number / shop_domain / product_name を含めて warn を出す                                                                                        |
| 3   | P1     | パフォーマンス                    | 998  | 二重ループ内で毎回 `SELECT ... FROM items WHERE order_id=?` を発行。候補 order_id 群の items をまとめて取得しメモリ上でマッチングする                                                           |
| 4   | P2     | orders_to_delete の重複           | 971  | 同一 order_id が複数回 push され得るため、後段の COUNT/DELETE が重複実行。HashSet で重複排除してから処理                                                                                        |
| 5   | P1     | 数量減算のテスト未追加            | 1056 | `UPDATE items SET quantity = ?`（new_qty > 0）の分岐が未検証。例: 元が2個で組み換え後が1個 → 1個に減算するケースのテスト追加                                                                    |

---

## 対応方針

### 推奨: レビュー指摘をすべて解消してからマージ

- P1 指摘 4 件 + P2 指摘 1 件を修正
- 工数は中程度、1PRで完結

---

## 実施タスク

### 1. change_email_internal_date の活用（P1）

| 項目 | 内容                                                                                                  |
| ---- | ----------------------------------------------------------------------------------------------------- |
| 対象 | `repository.rs` apply_change_items                                                                    |
| 内容 | 候補注文取得の SQL に `change_email_internal_date` を使った絞り込みを追加                             |
| 条件 | `order_date < change_email_internal_date` 相当（order_date が NULL の場合は created_at または考慮外） |

**スキーマ**: `orders` に `order_date` (DATETIME)、`created_at` (DATETIME) あり。`internal_date` は UTC ミリ秒（i64）。`order_date` は日時文字列（例: "2024-01-01 12:00:00"）で保存。

**SQL 修正案**（`internal_date` を日時文字列に変換して比較）:

```sql
-- change_email_internal_date が Some の場合のみ追加
AND (
  ? IS NULL
  OR o.order_date IS NULL
  OR o.order_date < datetime(? / 1000, 'unixepoch', 'localtime')
)
```

※ SQLite の DATETIME 比較は文字列辞書順で可能。`order_date` が ISO 形式であれば正確に比較できる。`created_at` で代替する場合は `COALESCE(o.order_date, o.created_at)` を使用。

### 2. マッチ無し時の warn ログ（P1）

| 項目 | 内容                                                                                                                                                     |
| ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 対象 | `repository.rs` apply_change_items の `order_loop` 内                                                                                                    |
| 内容 | `order_loop` を抜けた際に1件もマッチしなかった場合、`log::warn!` を出力                                                                                  |
| 例   | `log::warn!("apply_change_items: no matching order for item {:?} shop_domain={:?} order_number={}", product_name, shop_domain, order_info.order_number)` |

### 3. パフォーマンス改善（P1）

| 項目 | 内容                                                                                          |
| ---- | --------------------------------------------------------------------------------------------- |
| 対象 | `repository.rs` apply_change_items                                                            |
| 内容 | 候補 order_ids に対して items を一括取得（`WHERE order_id IN (...)`）し、メモリ上でマッチング |
| 実装 | ループ前に `HashMap<order_id, Vec<Item>>` を構築し、`order_loop` 内ではメモリ上のデータを参照 |

### 4. orders_to_delete の重複排除（P2）

| 項目 | 内容                                                                           |
| ---- | ------------------------------------------------------------------------------ |
| 対象 | `repository.rs` apply_change_items                                             |
| 内容 | `orders_to_delete: Vec<i64>` → `HashSet<i64>` に変更し、重複を排除             |
| 後段 | `for order_id in orders_to_delete` は `HashSet` のイテレートでそのまま利用可能 |

### 5. 数量減算のテスト追加（P1）

| 項目 | 内容                                                             |
| ---- | ---------------------------------------------------------------- |
| 対象 | `repository.rs` mod tests                                        |
| 内容 | 元注文に商品A×2個があり、組み換え後が1個のケースを追加           |
| 検証 | `UPDATE items SET quantity = 1` となり、order は削除されないこと |

**テスト例**:

```rust
#[tokio::test]
async fn test_apply_change_items_reduces_quantity() {
    // 元注文に商品A が2個
    // 組み換え後は商品A が1個
    // → 元注文の quantity が 2 -> 1 に減算され、order は残る
}
```

---

## 実行フロー

1. 上記タスク 1〜5 を順に実施
2. `cargo test -p tauri-app repository` でテスト実行
3. `cargo test` / `npm test` で全体テスト
4. コミット・プッシュ
5. CI 通過を確認
6. Copilot レビュー再依頼

---

## マージ前チェックリスト

- [ ] Task 1: change_email_internal_date で候補注文を絞り込み
- [ ] Task 2: マッチ無し時に warn ログを出力
- [ ] Task 3: items を一括取得してパフォーマンス改善
- [ ] Task 4: orders_to_delete を HashSet で重複排除
- [ ] Task 5: 数量減算（new_qty>0）のテスト追加
- [ ] `cargo test -p tauri-app repository` 成功
- [ ] `cargo test` / `npm test` 成功
- [ ] CI 通過
- [ ] Copilot レビュー再依頼

---

## 備考

- `docs/issue-27-plan.md` の Phase 3.2 に「時系列・ショップ絞り込み（order_date < 組み換えメール日時）」が既に設計済み。今回の P1 指摘はその実装漏れを指摘している。
- `orders` テーブルのカラム定義を確認してから `change_email_internal_date` の絞り込み条件を実装すること。
