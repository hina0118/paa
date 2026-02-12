# データのバックアップ・復元

PAA のメタデータ（AI解析済み商品データ、画像キャッシュ、店舗設定など）をバックアップ・復元する機能の使い方です。

## バックアップ対象

以下のテーブルと画像ファイルがエクスポートされます。

| テーブル        | 内容                                                             |
| --------------- | ---------------------------------------------------------------- |
| images          | 商品画像のメタデータ（item_name_normalized と file_name の対応） |
| shop_settings   | 店舗設定（送信元アドレス、パーサータイプなど）                   |
| product_master  | AI（Gemini）解析済みの商品情報（メーカー、シリーズ、スケール等） |
| emails          | 取得済みメール（message_id、本文、差出人、件名など）             |
| item_overrides  | 商品データの補正値（手動補正）                                   |
| order_overrides | 注文データの補正値（手動補正）                                   |
| excluded_items  | 商品の論理削除（非表示）                                         |
| excluded_orders | 注文の論理削除（非表示）                                         |

画像ファイルは `app_data_dir/images/` に保存されている実体が ZIP 内の `images/` フォルダに含まれます。

## 使い方

1. サイドバーから「データのバックアップ」をクリック
2. **データのバックアップ**ボタンでエクスポート、**データのインポート**ボタンでZIPを選択してインポート、**復元（復元ポイント）**ボタンで同一PC内の復元ポイントから復元

### エクスポート（バックアップ）

1. 「データのバックアップ」ボタンをクリック
2. 保存先を選択するダイアログが表示される
3. ファイル名を指定して保存（デフォルト: `paa_export_YYYYMMDD_HHmmss.zip`）
4. ZIP ファイルが作成される
5. 同一PCでの復元を容易にするため、ZIP は **復元ポイント**として `app_data_dir/paa_restore_point.zip` にも保存される

### インポート（ZIPを選択）

1. 「データのインポート」ボタンをクリック
2. 確認ダイアログで「Ok」をクリック
3. バックアップ ZIP を選択
4. データが現在の DB にマージされる（INSERT OR IGNORE のため、既存データと競合する行はスキップ）
5. 選択した ZIP で **復元ポイント**（`app_data_dir/paa_restore_point.zip`）が更新される

### 復元（復元ポイント）

1. 「復元（復元ポイント）」ボタンをクリック
2. 確認ダイアログで「Ok」をクリック
3. `app_data_dir/paa_restore_point.zip` から復元される（ファイル選択は不要）
4. データが現在の DB にマージされる（INSERT OR IGNORE のため、既存データと競合する行はスキップ）

## ZIP 内構成

```
paa_export_YYYYMMDD_HHmmss.zip
├── manifest.json       # バージョン・エクスポート日時
├── images.json         # images テーブル
├── shop_settings.json  # shop_settings テーブル
├── product_master.json # product_master テーブル
├── emails.ndjson       # emails テーブル（NDJSON形式、旧形式は emails.json も読み込み可）
├── item_overrides.json  # item_overrides テーブル
├── order_overrides.json # order_overrides テーブル
├── excluded_items.json  # excluded_items テーブル
├── excluded_orders.json # excluded_orders テーブル
└── images/             # 画像ファイル
    ├── xxx.jpg
    └── yyy.png
```

## 注意事項

- DB をリセットしても、バックアップからメタデータを復元することで AI 解析済みの商品データや画像キャッシュを維持できます
- インポート時、UNIQUE 制約で競合する行は既存データが維持されます
- 画像ファイルは同名が存在する場合はスキップ（既存を維持）
