# Issue #27 対応計画: 組み換えメール処理の改善（元注文商品の自動キャンセル処理）

## 1. 概要

**Issue**: [#27 組み換えメール処理の改善: 元注文商品の自動キャンセル処理](https://github.com/hina0118/paa/issues/27)

現在の組み換えメールパーサー (hobbysearch_change / hobbysearch_change_yoyaku) は新注文を登録するのみで、元注文の商品が重複して残る問題があります。商品情報メインのアプリケーションとして、元注文の商品を自動的にキャンセル（削除）する処理を実装する必要があります。

## 2. 現状分析

### 2.1 現在の組み換えメール処理

| 処理   | 内容                                                                     |
| ------ | ------------------------------------------------------------------------ |
| パース | `hobbysearch_change` / `hobbysearch_change_yoyaku` で OrderInfo を抽出   |
| 保存   | `save_order` で order_number + shop_domain をキーに upsert、items を追加 |
| 問題   | **元注文の商品が削除されず残る** → 同一商品が複数注文に重複表示          |

### 2.2 組み換えの仕様（ホビーサーチ）

1. 複数の注文（3つ以上もあり得る）から一つの注文にまとめる機能
2. 元注文の一部の商品のみを組み替えることも可能。この時元注文は削除されず残る
3. 元注文の全商品が削除されたときに元注文自体が無効となり削除される
4. **商品名はそのまま引き継がれる** → 商品名で元注文を検索可能
5. 組み換えは過去のメールから解析するため、`internal_date` 昇順でパースしている現状は正しい
6. **発送連絡が来た注文は組み換え対象外**
7. メールには**組み換え後の新しい注文番号**のみが含まれる（元注文番号は含まれない）

### 2.3 関連実装

| 項目       | ファイル                                                        | 備考                                 |
| ---------- | --------------------------------------------------------------- | ------------------------------------ |
| パーサー   | `parsers/hobbysearch_change.rs`, `hobbysearch_change_yoyaku.rs` | OrderInfo を返す                     |
| 保存       | `repository.rs` `save_order`                                    | order_number + shop_domain で upsert |
| キャンセル | `repository.rs` `apply_cancel`                                  | 注文番号 + 商品名で商品削除・減算    |
| 発送状態   | `deliveries.delivery_status`                                    | `shipped` 等                         |
| パース順序 | `get_unparsed_emails` → `ORDER BY internal_date ASC`            | 過去から順に処理                     |

### 2.4 データベース

- `orders`: order_number, shop_domain, order_date 等（deleted 列は未実装）
- `items`: order_id, item_name, item_name_normalized, brand, quantity 等
- `deliveries`: order_id, delivery_status（shipped = 発送済み）
- 発送済み判定: `deliveries.delivery_status IN ('shipped', 'in_transit', ...)` で該当 order は組み換え対象外

## 3. 実装方針

### 3.1 処理フロー

1. 組み換えメールをパース → 新注文番号 + 商品リスト取得
2. **元注文の商品を削除**（apply_change_items 相当の処理）
   - 新注文の各商品について、同じショップの過去注文（発送済みでない）から商品名でマッチする item を検索
   - 該当 item を削除（quantity 減算または DELETE）
   - 残り商品数が 0 になった注文は orders から削除（または deleted フラグ）
3. 新注文の商品を登録（既存の save_order）

### 3.2 元注文の特定方法

1. **商品情報ベース**（主要）
   - 新注文の各商品（item_name, item_name_normalized）で過去の items を検索
   - apply_cancel と同様のマッチング: 完全一致 → 包含 → 正規化部分一致

2. **絞り込み条件**
   - 同じショップ (shop_domain)
   - 発送済みでない（deliveries に shipped 系がない、または order に deliveries が未紐付け）
   - 時系列: 注文日または created_at が組み換えメールの internal_date より前

3. **重複商品への対応**
   - 同じ注文内に同一商品名が複数ある場合、個数・単価も考慮してマッチング
   - まずは「商品名 + order_id で最初にマッチした 1 件」から開始し、必要に応じて拡張

### 3.3 apply_cancel との使い分け

- **apply_cancel**: キャンセルメール（注文番号 + 商品名 + キャンセル個数が明示）→ 既存実装をそのまま利用
- **apply_change**: 組み換えメール（新注文番号 + 商品リストのみ、元注文番号は不明）→ 商品名で元注文を逆引きして削除

## 4. 対応計画

### Phase 1: リポジトリ拡張

| #   | タスク                            | ファイル        | 内容                                                                   |
| --- | --------------------------------- | --------------- | ---------------------------------------------------------------------- |
| 1.1 | `apply_change_items` メソッド追加 | `repository.rs` | 新注文の商品リストを受け取り、同じショップの過去注文から該当商品を削除 |
| 1.2 | 発送済み注文の判定                | `repository.rs` | 組み換え対象外とするため、deliveries で shipped 系の注文を除外         |
| 1.3 | 空注文の削除                      | `repository.rs` | 全商品削除後に items が 0 件になった order を削除（または論理削除）    |

### Phase 2: バッチパース統合

| #   | タスク                | ファイル                              | 内容                                                                                                       |
| --- | --------------------- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| 2.1 | change 処理の分岐     | `parsers/mod.rs`, `batch_commands.rs` | hobbysearch_change / hobbysearch_change_yoyaku の場合、save_order の**前**に apply_change_items を呼ぶ     |
| 2.2 | email_parse_task 対応 | `parsers/email_parse_task.rs`         | 同様に apply_change_items を呼ぶ                                                                           |
| 2.3 | 冪等性の確保          | 同上                                  | 同じメールを 2 回処理しても結果が変わらないよう、order_emails 紐付け後に apply_change_items の対象から除外 |

### Phase 3: 元注文検索ロジック

| #   | タスク                   | ファイル        | 内容                                                                    |
| --- | ------------------------ | --------------- | ----------------------------------------------------------------------- |
| 3.1 | 商品マッチング           | `repository.rs` | apply_cancel のマッチングロジック（完全一致→包含→正規化）を流用         |
| 3.2 | 時系列・ショップ絞り込み | `repository.rs` | shop_domain 一致、order_date < 組み換えメール日時、発送済みでない       |
| 3.3 | 複数候補時の優先順位     | `repository.rs` | 複数注文に同じ商品がある場合、最も最近の注文（order_date DESC）から優先 |

### Phase 4: テスト・検証

| #   | タスク         | 内容                                                            |
| --- | -------------- | --------------------------------------------------------------- |
| 4.1 | 単体テスト     | apply_change_items のユニットテスト（repository 内）            |
| 4.2 | 統合テスト     | 組み換えメール → 元注文検索 → 商品削除 → 新注文登録の一連フロー |
| 4.3 | サンプルデータ | 組み換え前後の注文を再現したテストデータ                        |
| 4.4 | フォールバック | 元注文が見つからない場合も新注文は登録し、警告ログを残す        |

## 5. 詳細仕様

### 5.1 apply_change_items シグネチャ（案）

```rust
/// 組み換えメールに含まれる商品を元注文から削除する。
/// 新注文の各商品について、同じショップの過去注文（発送済みでない）から
/// 商品名でマッチする item を検索し、削除または quantity 減算する。
/// 残り商品が 0 になった order は削除する。
async fn apply_change_items(
    &self,
    order_info: &OrderInfo,
    shop_domain: Option<String>,
    change_email_internal_date: Option<i64>,
) -> Result<(), String>;
```

### 5.2 発送済み判定

```sql
-- 発送済み注文を除外
SELECT o.id FROM orders o
LEFT JOIN deliveries d ON d.order_id = o.id
WHERE o.shop_domain = ?
  AND (d.id IS NULL OR d.delivery_status NOT IN ('shipped', 'in_transit', 'out_for_delivery', 'delivered'))
```

### 5.3 商品マッチング

- apply_cancel と同様: `strip_bracketed_content`, `item_name` / `item_name_normalized` による比較
- 新注文の商品が元注文の「どの item に対応するか」を 1:1 で特定

### 5.4 空注文の削除

- 商品削除後に `SELECT COUNT(*) FROM items WHERE order_id = ?` が 0 の場合、`DELETE FROM orders WHERE id = ?`
- order_emails, deliveries は CASCADE または明示削除

## 6. 実装上の注意点（issue 27 より）

| 項目           | リスク                               | 対応                                                      |
| -------------- | ------------------------------------ | --------------------------------------------------------- |
| 商品名の不一致 | メールの改行・表記ゆれ               | トリミング、正規化、部分一致で許容                        |
| 重複商品       | 同一注文内に同一商品が複数           | 個数・単価も考慮したマッチング（初期は 1 件マッチで開始） |
| 発送済みロック | 発送後に組み換え変更が来る           | deliveries で shipped 系の注文は変更対象外                |
| 冪等性         | 同じメールを 2 回処理                | internal_date 昇順 + order_emails 紐付けで重複防止        |
| 元注文なし     | 古いメール削除等で元注文が DB にない | 新注文は登録し、警告ログを残す                            |

## 7. 実装順序

1. **Phase 1**: apply_change_items 実装（repository.rs）
2. **Phase 2**: バッチパース・email_parse_task での呼び出し
3. **Phase 3**: マッチング精度の調整（必要に応じて）
4. **Phase 4**: テスト追加・検証

## 8. 実装状況（2026-02-08）

- [x] Phase 1: `apply_change_items` 実装（repository.rs）
- [x] Phase 2: `email_parse_task.rs` で apply_change_items 呼び出し
- [x] Phase 2: `parsers/mod.rs`（batch_parse_emails）で apply_change_items 呼び出し
- [x] Phase 4: 統合テスト 3 件追加（repository.rs）

## 9. 見積もり

- Phase 1: 約 2-3 時間
- Phase 2: 約 1-2 時間
- Phase 3: 約 1 時間（マッチングロジック調整）
- Phase 4: 約 1-2 時間

**合計**: 約 5-8 時間

## 10. 関連

- #26 組み換えメールパーサー実装（完了）
- キャンセルメール設計: `docs/plans/hobbysearch-cancel-mail-design.md`
- apply_cancel: `repository.rs` L656 付近
