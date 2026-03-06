# Issue #245: 配達完了メールのパースと配送状況確認の組み合わせ

## 概要

配達完了メール（「お届けが完了しました」等の通知）をパースし、`deliveries.delivery_status` を `delivered` に更新する。これにより配送状況確認バッチの対象が減り、配送業者サイトへのHTTPアクセスを削減できる。

## 方針

パース時にメールとHTMLの両方を解析し、結果を統合する。メールが来ていない場合はHTMLで補完し、HTMLが取得できない場合はメールで補完する。

## 背景

- **配達完了メール**: 店舗や配送業者から送られる「お届けが完了しました」系の通知メール
- **配送状況確認**: 追跡番号で配送業者サイトにHTTPアクセスし、HTMLをパースして `delivery_status` を更新
- 配達完了メールをパースすれば、配送業者サイトへのアクセスなしで `delivered` を反映可能

## 目的

1. メールパース時に配達完了を検出し、`delivery_status = 'delivered'` に即時更新
2. 配達完了メールが来たものは配送状況確認の対象外（既存クエリで自動スキップ）
3. 配送業者サイトへのHTTPアクセス削減

## 対応内容

### 1. 配達完了メールパーサーの追加

- `parser_type`: `xxx_delivered`（店舗別）または `delivery_complete`（共通）を検討
- 検出キーワード: お届け済み / 配達完了 / お届けが完了 等（`delivery_check/mod.rs` の `delivered_keywords` と同様）
- 抽出: 注文番号、追跡番号、配達日時

### 2. 処理フロー

- 既存 order を注文番号で検索
- 該当 order の deliveries の `delivery_status` を `'delivered'` に更新
- `actual_delivery` に配達日時を設定（パース可能な場合）

### 3. 配送状況確認との連携

- 既存の配送状況確認は `delivery_status NOT IN ('delivered', ...)` で対象取得
- パースで `delivered` になったものは自動的にスキップ → **変更不要**

## タスク

- [ ] 配達完了メールのサンプル収集・分析
- [ ] パーサー種別設計（店舗別 vs 共通）
- [ ] 配達完了メールパーサー実装（1店舗以上）
- [ ] delivery_status 更新ロジック
- [ ] shop_settings への parser_type 登録
- [ ] 他店舗・配送業者への拡張

## 参考

- `docs/issue-243-delivery-status-parse-integration.md`（詳細設計）
- `src-tauri/src/delivery_check/mod.rs`（delivered_keywords）
- `src-tauri/src/plugins/hobbysearch/parsers/send.rs`（既存 send パーサー例）
