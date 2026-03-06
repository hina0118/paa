# Issue #246: ふるいちオンライン メールパース対応

## 概要

ふるいちオンライン（furu1.online）の注文確認メール・発送通知メールをパースし、orders / items / deliveries に登録する。既存のホビーサーチ・DMM・プレミアムバンダイ等と同様に、VendorPlugin として実装する。

## 背景

- **ふるいちオンライン**: 駿河屋が運営する通販サイト（furu1.online）
- **メール形式**: Cuenote 経由で送信、From: info@furu1.online
- **サンプル配置**: `sample/【ふるいちオンライン】 ご注文ありがとうございます.eml`, `sample/【ふるいちオンライン】商品発送のお知らせ.eml`

## メールサンプル分析

### 1. 注文確認メール（ご注文ありがとうございます）

| 項目                      | 値                                                |
| ------------------------- | ------------------------------------------------- |
| Subject                   | 【ふるいちオンライン】 ご注文ありがとうございます |
| From                      | info@furu1.online                                 |
| Content-Type              | text/plain; charset=utf-8                         |
| Content-Transfer-Encoding | quoted-printable                                  |

**本文フォーマット（デコード後）**:

```
ご注文番号：100409780
ご注文日：2026-03-03 22:25:08
ご注文者名：山田太郎
お支払い方法：Amazon Pay
---------------------------------------------
お届け先
〒1000001
東京都千代田区丸の内1-1-1 テストマンション101号
Tel：09000000000
山田太郎様
---------------------------------------------
ご注文商品：
03ゼウスⅠ　カルノージャート:1個
030カルノージャート　エクサ:1個
---------------------------------------------
商品小計（税込）「6,158」円
送料(税込)「0」円
クーポン利用「0」円
ポイント利用「0」ポイント
---------------------------------------------
ご注文金額合計（税込）「6,158」円
```

**抽出対象**:

- 注文番号: `ご注文番号：(\d+)`
- 注文日: `ご注文日：(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})`
- 配送先: お届け先セクション（郵便番号・住所・宛名・電話番号）
- 商品: `ご注文商品：` 以降の行（`商品名:数量` 形式）
- 金額: 商品小計・送料・合計（`「(\d+)」円` 形式）

### 2. 発送通知メール（商品発送のお知らせ）

| 項目         | 値                                       |
| ------------ | ---------------------------------------- |
| Subject      | 【ふるいちオンライン】商品発送のお知らせ |
| From         | info@furu1.online                        |
| Content-Type | text/plain; charset=utf-8                |

**本文フォーマット（発送内容セクション）**:

```
発送内容
---------------------------------------------
配送会社：ゆうパケット
伝票番号：680156937342
---------------------------------------------
（以下、注文確認と同様のご注文内容）
```

**抽出対象**:

- 配送会社: `配送会社：(.+)`
- 伝票番号: `伝票番号：(\d+)`
- 注文番号・商品・金額: 注文確認と同様のセクション

**配送業者マッピング**:

- ゆうパケット → 既存 `delivery_check` で対応済み（日本郵便追跡）
- 佐川急便等の URL もメール内に記載あり

## 対応内容

### 1. プラグイン追加

- **プラグイン名**: `FuruichiOnlinePlugin`（ふるいちオンライン）
- **parser_type**:
  - `furuichi_confirm`: 注文確認メール
  - `furuichi_send`: 発送通知メール

### 2. default_shop_settings

| shop_name          | sender_address    | parser_type      | subject_filters                                   |
| ------------------ | ----------------- | ---------------- | ------------------------------------------------- |
| ふるいちオンライン | info@furu1.online | furuichi_confirm | 【ふるいちオンライン】 ご注文ありがとうございます |
| ふるいちオンライン | info@furu1.online | furuichi_send    | 【ふるいちオンライン】商品発送のお知らせ          |

※ subject にスペース有無のバリエーションがある場合は複数登録または正規表現で対応を検討

### 3. パーサー実装

#### furuichi_confirm

- **参照**: `hobbysearch/parsers/confirm.rs`, `premium_bandai/parsers/confirm.rs`
- **抽出**:
  - 注文番号: `ご注文番号：(\d+)`
  - 注文日: `ご注文日：` の値（YYYY-MM-DD HH:MM:SS）
  - 配送先: お届け先セクション（郵便番号・住所・宛名・電話）
  - 商品: `ご注文商品：` 以降、`商品名:数量` 形式（改行区切り）
  - 金額: 商品小計・送料・合計
- **delivery_info**: None（注文確認時点では配送情報なし）

#### furuichi_send

- **参照**: `hobbysearch/parsers/send.rs`, `dmm/parsers/send.rs`
- **抽出**:
  - 注文番号・商品・金額: confirm と同様
  - 配送会社: `配送会社：ゆうパケット` 等
  - 伝票番号: `伝票番号：(\d+)`
- **delivery_info**: carrier, tracking_number を設定
- **carrier_url**: ゆうパケットの場合は日本郵便追跡 URL を設定可能（`delivery_check` の `build_tracking_url` と整合）

### 4. 商品行パース仕様

サンプルの商品行形式:

```
03ゼウスⅠ　カルノージャート:1個
030カルノージャート　エクサ:1個
```

- パターン: `(.+):(\d+)個` で商品名と数量を抽出
- 全角スペース（　）は商品名に含める
- 単価・小計はメールに含まれないため、商品小計から按分するか、`unit_price`/`subtotal` を None にするか検討（他プラグインの扱いを参照）

### 5. モジュール構成

```
src-tauri/src/plugins/
├── mod.rs          # pub mod furuichi_online を追加
└── furuichi_online/
    ├── mod.rs      # FuruichiOnlinePlugin, inventory::submit!
    └── parsers/
        ├── mod.rs
        ├── confirm.rs
        └── send.rs
```

### 6. internal_date の扱い

- confirm: 注文日がメール本文にあるため `order_date` に使用。無い場合は `internal_date` で補完（`apply_internal_date`）
- send: 注文日は confirm と同様。配送情報は `delivery_info` に含める

## 配送業者との連携

- **ゆうパケット**: `delivery_check/mod.rs` の `delivered_keywords` および `build_tracking_url` で既に「ゆうパケット」に対応済み
- 日本郵便追跡: `https://trackings.post.japanpost.jp/services/srv/search/input`
- 佐川急便: `http://k2k.sagawa-exp.co.jp/p/sagawa/web/okurijoinput.jsp`
- 発送通知で carrier を「ゆうパケット」として登録すれば、既存の配送状況確認がそのまま動作する

## タスク

- [ ] `furuichi_online` プラグインモジュール作成
- [ ] `furuichi_confirm` パーサー実装（注文番号・注文日・配送先・商品・金額）
- [ ] `furuichi_send` パーサー実装（confirm に加え配送会社・伝票番号）
- [ ] `FuruichiOnlinePlugin` の `dispatch` 実装（confirm/send 共通フロー）
- [ ] `default_shop_settings` 登録（info@furu1.online × 2 種別）
- [ ] `plugins/mod.rs` に `pub mod furuichi_online` 追加
- [ ] サンプルメールでのパーステスト（手動確認またはユニットテスト）
- [ ] `mod.rs` の `test_all_*_parser_types_have_plugin` に furuichi を追加

## 参考

- 既存プラグイン: `hobbysearch`, `premium_bandai`, `dmm`
- 配送状況確認: `src-tauri/src/delivery_check/mod.rs`
- メールパースフロー: `src-tauri/src/parsers/email_parse_task.rs`
- サンプル: `sample/【ふるいちオンライン】 ご注文ありがとうございます.eml`, `sample/【ふるいちオンライン】商品発送のお知らせ.eml`
