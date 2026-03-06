# Issue #243: 配達完了メールのパースと配送状況確認の組み合わせ

## 方針: パース時にメール・HTMLを解析し統合する

**パース時にメールとHTMLの両方を解析し、結果を統合する**という方針とする。

| 情報源         | 内容                                           | 特徴                                             |
| -------------- | ---------------------------------------------- | ------------------------------------------------ |
| **メール解析** | 注文確認・発送通知・配達完了メール等           | 自動取得（Gmail同期）、リアルタイム性が高い      |
| **HTML解析**   | 購入履歴ページ（駿河屋等）、配送業者追跡ページ | 手動保存またはクロール、取引状況・追跡詳細を補完 |

- パースフェーズでメール・HTMLの両方を解析
- 得られた情報を order / delivery に統合して反映
- メールが来ていない場合はHTMLで補完、HTMLが取得できない場合はメールで補完

## 背景

- **issue 243**: 配達完了メール（「お届けが完了しました」等の通知）をパースし、`deliveries.delivery_status` を `delivered` に更新する
- **配送状況確認バッチ**: `deliveries` テーブルの追跡番号を用いて配送業者サイトにHTTPアクセスし、HTMLをパースして `delivery_status` を更新する
- 配達完了メールをパースすれば、配送業者サイトへのアクセスなしで `delivery_status = 'delivered'` を反映できる

## 目的

配達完了メールパースと配送状況確認を組み合わせ、以下を実現する：

1. **パース時に配達完了を反映**: メールから「お届け完了」等を検出し、`deliveries.delivery_status` を `delivered` に即時更新
2. **配送業者サイトアクセスの削減**: 配達完了メールが来たものは追跡サイトへのHTTPリクエストをスキップ
3. **データの一貫性**: メールパースで得た配達完了情報を優先し、配送状況確認は補完的に実行

## 現状のデータフロー

```
[メール同期] → [メールパース] → orders, items, deliveries 作成
     │              │              (confirm/send 等で delivery_status 設定)
     │              │
     └──────────────┴───────────────────────────────────────────
                                    ↓
[配送状況確認] ← deliveries (tracking_number, carrier, delivery_status NOT IN delivered)
     ↓
配送業者サイトにHTTP → HTMLパース → delivery_status 更新
```

## 提案するデータフロー

```
[メール同期] → [メールパース]
     │              │
     │              ├─ confirm / send 等 → deliveries 作成（shipped, in_transit 等）
     │              │
     │              └─ 配達完了メール（xxx_delivered）→ 既存 delivery の delivery_status = 'delivered' に更新
     │
     └───────────────────────────────────────────────────────────
                                    ↓
[配送状況確認] ← delivery_status NOT IN ('delivered', 'cancelled', 'returned') のもののみ
     ↓
配送業者サイトにHTTP → 詳細ステータス取得（配達完了メールが来ていないもの）
```

## 対応内容

### 1. 配達完了メールパーサーの追加

各店舗・配送業者から送られる「お届けが完了しました」系メールをパースする `parser_type` を追加。

**検出キーワード例**（メール本文）:

- お届け済み / お届けが完了 / 配達完了 / お届けしました
- 配送業者サイトのHTMLパースで使用しているキーワードと同様（`delivery_check/mod.rs` の `delivered_keywords` 参照）

**抽出対象**:

- 注文番号（order 紐付け用）
- 追跡番号（delivery 特定用、複数配送がある場合）
- 配達日時（`deliveries.actual_delivery` に反映可能）

**処理**:

- 既存 order を注文番号で検索
- 該当 order の deliveries の `delivery_status` を `'delivered'` に更新
- `actual_delivery` に配達日時を設定（パース可能な場合）

### 2. パーサー種別の設計

| 案                | 内容                                                                    | 備考                                 |
| ----------------- | ----------------------------------------------------------------------- | ------------------------------------ |
| A. 店舗別パーサー | `hobbysearch_delivered`, `dmm_delivered` 等を各プラグインに追加         | 既存プラグイン構造に沿う             |
| B. 共通パーサー   | `delivery_complete` 等の汎用パーサーを1つ用意し、件名・送信元で店舗判定 | 配送業者からのメールにも対応しやすい |

**推奨**: 店舗・配送業者ごとにメール形式が異なるため、案 A を基本としつつ、配送業者（ヤマト・佐川等）からの共通メール用に案 B を併用する構成も検討。

### 3. 配送状況確認バッチとの連携

現在の配送状況確認は以下の条件で対象を取得している：

```sql
SELECT id, tracking_number, carrier FROM deliveries
WHERE delivery_status NOT IN ('delivered', 'cancelled', 'returned')
  AND tracking_number IS NOT NULL AND TRIM(tracking_number) != ''
  AND carrier IS NOT NULL AND TRIM(carrier) != ''
```

**配達完了メールパース後**:

- パースで `delivery_status = 'delivered'` に更新された delivery は、上記 `WHERE` により自動的に配送状況確認の対象外になる
- **変更不要**: 既存の配送状況確認バッチはそのままで、パースで配達完了になったものは自然にスキップされる

### 4. パイプライン上の位置づけ

- **メールパース**: 配達完了メールも `get_unparsed_emails` で取得され、`internal_date` 昇順で他のメールと一緒にパースされる
- **配送状況確認**: パイプラインの Step 4 として、メールパース完了後に実行。この時点で配達完了メール由来の `delivered` は既に反映済み
- **実行順序**: `メール同期 → メールパース（配達完了メール含む）→ 商品名解析 → 配送状況確認`

### 5. 既存パーサーとの関係

| パーサー          | 役割             | delivery_status                          |
| ----------------- | ---------------- | ---------------------------------------- |
| xxx_confirm       | 注文確認         | 新規 delivery 作成（not_shipped 等）     |
| xxx_send          | 発送通知         | 追跡番号・carrier 追加、shipped 等に更新 |
| **xxx_delivered** | **配達完了通知** | **delivered に更新**                     |

## 技術的検討事項

1. **order 紐付け**: 配達完了メールに注文番号が含まれない場合（追跡番号のみ等）、`deliveries.tracking_number` で検索して紐付ける
2. **複数配送**: 1 order に複数 delivery がある場合、追跡番号で特定。追跡番号が無い場合は全 delivery を delivered にするか、仕様を検討
3. **重複パース**: 同一メールの再パース時は冪等に（既に delivered ならスキップ）

## タスク分解（案）

| #   | タスク                                                         | 優先度 |
| --- | -------------------------------------------------------------- | ------ |
| 1   | 配達完了メールのサンプル収集・分析（各店舗・配送業者の形式）   | P0     |
| 2   | パーサー種別設計（店舗別 vs 共通）の決定                       | P0     |
| 3   | 配達完了メールパーサー実装（1店舗以上）                        | P0     |
| 4   | delivery_status 更新ロジック（既存 order/delivery との紐付け） | P0     |
| 5   | shop_settings への parser_type 登録                            | P1     |
| 6   | 配送状況確認との連携確認（パース後は対象外になること）         | P1     |
| 7   | 他店舗・配送業者へのパーサー拡張                               | P2     |

## 参考

- 配送状況確認（HTMLキーワード）: `src-tauri/src/delivery_check/mod.rs` の `delivered_keywords`
- メールパースフロー: `src-tauri/src/parsers/email_parse_task.rs`
- 既存 send パーサー例: `src-tauri/src/plugins/hobbysearch/parsers/send.rs`, `src-tauri/src/plugins/dmm/parsers/send.rs`
