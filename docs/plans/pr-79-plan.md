# PR #79 対応計画

**作成日**: 2026-02-08  
**更新日**: 2026-02-08（新規レビュー3件追加）  
**PR**: [#79 feat: 組み換えメール処理の改善 - 元注文商品の自動キャンセル処理 (#27)](https://github.com/hina0118/paa/pull/79)  
**ブランチ**: `issue-27` → `main`  
**未解決レビュー**: 0件（P1: 0件）※旧5件+新規3件の計8件は対応済み

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
- **テスト**: 統合テスト 9 件追加（apply_change_items 系 7 件 + apply_change_items_and_save_order 系 2 件。商品削除・発送済み除外・マッチ無し・数量減算・複数注文跨ぎをカバー）

**変更ファイル**:

- `docs/issue-27-plan.md`（新規）
- `src-tauri/src/parsers/email_parse_task.rs`
- `src-tauri/src/parsers/mod.rs`
- `src-tauri/src/repository.rs`

---

## レビューコメント一覧

### 対応済み（Task 1〜5）

| #   | 優先度 | 指摘                              | 概要                                                 |
| --- | ------ | --------------------------------- | ---------------------------------------------------- |
| 1   | P1     | change_email_internal_date 未使用 | 候補注文取得 SQL に internal_date 絞り込みを追加     |
| 2   | P1     | マッチ無し時の warn ログ欠如      | order_loop で1件もマッチしなかった場合に warn を出力 |
| 3   | P1     | パフォーマンス                    | items を一括取得してメモリ上でマッチング             |
| 4   | P2     | orders_to_delete の重複           | HashSet で重複排除                                   |
| 5   | P1     | 数量減算のテスト未追加            | test_apply_change_items_reduces_quantity を追加      |

### 未対応（新規レビュー 3件）

| #   | 優先度 | 指摘                             | 行   | 概要                                                                                                                                                                                                                                                                                    |
| --- | ------ | -------------------------------- | ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 6   | P1     | マッチング判定の共通化           | 615  | `apply_change_items_in_tx` と `apply_cancel` の商品マッチング判定がほぼ同一。今後どちらかだけ修正されると挙動が乖離しやすい。マッチング判定を共通関数として切り出し、両方から利用する形にして保守性を上げる                                                                             |
| 7   | P1     | 同一トランザクションの統合テスト | 1191 | `apply_change_items_and_save_order` が change 系パーサーの実運用パスで使われているが、テストは `apply_change_items` 単体に留まっている。元注文削除と新注文保存が同一トランザクションで成立すること、保存失敗時にロールバックされることを検証する統合テストを追加                        |
| 8   | P1     | 同一注文内の複数行の消費漏れ     | 662  | 各 `order_id` につき `.find(...)` で最初の1件しか処理しない。同一注文内に同名商品が複数行ある場合、`remaining_qty` が残っても同じ注文内の次の行を消費できず、別注文に跨る/不要な warn が出る。`remaining_qty > 0` の間は同一 `order_id` の items を再検索して複数行から必要量を取り切る |

---

## 対応方針

### 推奨: レビュー指摘をすべて解消してからマージ

- Task 1〜5: 対応済み
- Task 6〜8: P1 指摘 3 件を修正
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

### 6. マッチング判定の共通化（P1）※新規

| 項目 | 内容                                                                                                                                         |
| ---- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| 対象 | `repository.rs` apply_change_items_in_tx / apply_cancel                                                                                      |
| 内容 | 商品マッチング判定（trim/括弧除去/normalized 比較）を共通関数として切り出し、`apply_change_items_in_tx` と `apply_cancel` の両方から利用する |
| 目的 | どちらかだけ修正されると挙動が乖離するリスクを防止し、保守性を向上                                                                           |

**実装方針**: `fn item_names_match(product_name: &str, item_name: &str, item_name_normalized: Option<&str>) -> bool` のようなヘルパーを追加し、両メソッドから呼び出す。

### 7. 同一トランザクションの統合テスト追加（P1）※新規

| 項目 | 内容                                                                                                                  |
| ---- | --------------------------------------------------------------------------------------------------------------------- |
| 対象 | `repository.rs` mod tests                                                                                             |
| 内容 | `apply_change_items_and_save_order` の統合テストを追加                                                                |
| 検証 | (1) 元注文削除と新注文保存が同一トランザクションで成立すること (2) 保存側が失敗した場合に削除がロールバックされること |

**テスト例**: 元注文をセットアップ → `apply_change_items_and_save_order` を呼び出し → 元注文の商品が削除され、新注文が保存されていることを確認。エラーシナリオでは save_order を失敗させる形でロールバックを検証。

### 8. 同一注文内の複数行の消費（P1）※新規

| 項目 | 内容                                                                                                                                                                   |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 対象 | `repository.rs` apply_change_items_in_tx の order_loop 内                                                                                                              |
| 問題 | 各 `order_id` につき `.find(...)` で最初の1件しか処理しない。同一注文内に同名商品が複数行ある場合、`remaining_qty` が残っても同じ注文内の次の行を消費できない          |
| 対応 | `remaining_qty > 0` の間、同一 `order_id` の items を再検索して複数行から必要量を取り切る。`loop { ... }` で囲み、1件処理した後に同じ order_id で再度 `.find()` を試す |

**Copilot 提案**: 同一 order_id 内で remaining_qty > 0 の間は `loop` で items を再検索し、マッチした行を順次消費。マッチがなくなったら `break` して次の order_id へ。

---

## 実行フロー

1. タスク 6〜8 を順に実施（タスク 1〜5 は対応済み）
2. `cargo test -p tauri-app repository` でテスト実行
3. `cargo test` / `npm test` で全体テスト
4. コミット・プッシュ
5. CI 通過を確認
6. Copilot レビュー再依頼

---

## マージ前チェックリスト

- [x] Task 1: change_email_internal_date で候補注文を絞り込み
- [x] Task 2: マッチ無し時に warn ログを出力
- [x] Task 3: items を一括取得してパフォーマンス改善
- [x] Task 4: orders_to_delete を HashSet で重複排除
- [x] Task 5: 数量減算（new_qty>0）のテスト追加
- [x] Task 6: マッチング判定を共通関数に切り出し
- [x] Task 7: apply_change_items_and_save_order の統合テスト追加
- [x] Task 8: 同一注文内の複数行を remaining_qty で消費するよう修正
- [ ] `cargo test -p tauri-app repository` 成功
- [ ] `cargo test` / `npm test` 成功
- [ ] CI 通過
- [ ] Copilot レビュー再依頼

---

## 備考

- `docs/issue-27-plan.md` の Phase 3.2 に「時系列・ショップ絞り込み（order_date < 組み換えメール日時）」が既に設計済み。今回の P1 指摘はその実装漏れを指摘している。
- `orders` テーブルのカラム定義を確認してから `change_email_internal_date` の絞り込み条件を実装すること。
- **Task 8**: Copilot が `loop` による同一 order_id 内の複数行消費のコード案をレビューコメント内で提案している。参照して実装可能。
