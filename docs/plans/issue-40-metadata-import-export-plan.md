# Issue #40 メタデータのインポート/エクスポート機能 実装計画

> **参照**: [GitHub Issue #40 feat:メタデータのインポート/エクスポート機能](https://github.com/hina0118/paa/issues/40)

**背景**: DB ファイルを削除・リセットしても、AI解析済みの商品データや画像キャッシュ情報などを維持できるようにする。

**Tech Stack**: Tauri 2.9.5, SQLite (sqlx), React, shadcn/ui

---

## 1. 対象テーブルとスキーマ

| テーブル         | 主なカラム                                                                   | UNIQUE 制約                   | 備考                                     |
| ---------------- | ---------------------------------------------------------------------------- | ----------------------------- | ---------------------------------------- |
| `images`         | id, item_name_normalized, file_name, created_at                              | (item_name_normalized)        | 画像は `app_data_dir/images/` に実体保存 |
| `shop_settings`  | id, shop_name, sender_address, parser_type, is_enabled, subject_filters, ... | (sender_address, parser_type) |                                          |
| `product_master` | id, raw_name, normalized_name, maker, series, ...                            | (raw_name)                    | Gemini AI 解析結果キャッシュ             |

---

## 2. エクスポート仕様

### 2.1 ZIP 内構成

```
paa_export_YYYYMMDD_HHmmss.zip
├── manifest.json          # バージョン・エクスポート日時
├── images.json            # images テーブル全行
├── shop_settings.json     # shop_settings テーブル全行
├── product_master.json    # product_master テーブル全行
└── images/                # 画像ファイル（file_name に対応）
    ├── xxx.jpg
    └── yyy.png
```

### 2.2 処理フロー

1. 3テーブルを SELECT して JSON にシリアライズ
2. `images` の `file_name` に対応するファイルを `app_data_dir/images/` から読み込み、`images/` に格納
3. Tauri のファイルダイアログで保存先を指定
4. 上記を ZIP にまとめて保存

---

## 3. インポート仕様

### 3.1 処理フロー

1. Tauri のファイルダイアログで ZIP を選択
2. ZIP を展開し、JSON を読み込み
3. ZIP 内のエントリ名を正規化し、`images/` ディレクトリ配下から外に出ないことを検証（`../` や絶対パスなどを含むものはスキップし、ログに記録）
4. 検証済みの `images/` 内の画像ファイルのみを、`app_data_dir/images/` 配下に収まるパスに対してコピー（上書き時はスキップ or 上書き方針を決定）

### 3.2 マージ方針

- **INSERT OR IGNORE** を使用し、UNIQUE 制約で競合する行はスキップ
- 画像ファイル: 同名が存在する場合はスキップ（既存を維持）

---

## 4. UI 仕様

### 4.1 配置

- **新規画面「データのバックアップ」** を追加
- サイドバーにメニュー項目を追加（例: アイコン `Archive` または `HardDrive`）
- 画面内の Card:
  - 「データのバックアップ」ボタン → エクスポート
  - 「データの復元」ボタン → インポート（確認ダイアログあり）
- 各ボタンに説明文を表示（何がエクスポート/インポートされるか）

### 4.2 画面構成

| 項目             | 内容                                         |
| ---------------- | -------------------------------------------- |
| コンポーネント   | `src/components/screens/backup.tsx`（新規）  |
| ルート ID        | `backup`                                     |
| サイドバー表示名 | 「データのバックアップ」                     |
| アイコン         | lucide-react の `Archive` または `HardDrive` |

### 4.3 フロー

- エクスポート: ボタンクリック → 保存ダイアログ → ZIP 保存 → 成功通知
- インポート: ボタンクリック → 確認ダイアログ → ファイル選択 → ZIP 読み込み → マージ → 成功通知

---

## 5. タスク一覧

### Phase 1: 基盤整備

| #   | タスク                   | 内容                                                | ファイル                                      |
| --- | ------------------------ | --------------------------------------------------- | --------------------------------------------- |
| 1.1 | tauri-plugin-dialog 追加 | ファイルダイアログ（保存・開く）                    | Cargo.toml, lib.rs, capabilities/default.json |
| 1.2 | zip crate 追加           | ZIP 作成・展開                                      | Cargo.toml                                    |
| 1.3 | エクスポートコマンド     | 3テーブル + 画像 → ZIP                              | lib.rs, 新規 export_metadata.rs               |
| 1.4 | インポートコマンド       | ZIP → JSON 読み込み + INSERT OR IGNORE + 画像コピー | lib.rs, 新規 import_metadata.rs               |

### Phase 2: フロントエンド

| #   | タスク               | 内容                                              | ファイル                                          |
| --- | -------------------- | ------------------------------------------------- | ------------------------------------------------- |
| 2.1 | ナビゲーション追加   | Screen 型・サイドバー・App.tsx に `backup` を追加 | navigation-context-value.ts, sidebar.tsx, App.tsx |
| 2.2 | バックアップ画面作成 | 新規画面コンポーネント「データのバックアップ」    | backup.tsx（新規）                                |
| 2.3 | エクスポート呼び出し | invoke + 成功/失敗通知                            | backup.tsx                                        |
| 2.4 | インポート呼び出し   | 確認ダイアログ + invoke + 成功/失敗通知           | backup.tsx                                        |

### Phase 3: テスト・ドキュメント

| #   | タスク             | 内容                                   | ファイル                               |
| --- | ------------------ | -------------------------------------- | -------------------------------------- |
| 3.1 | 単体テスト         | export/import ロジック                 | export_metadata.rs, import_metadata.rs |
| 3.2 | E2E テスト（任意） | バックアップ画面のボタン表示・クリック | backup.spec.ts（新規）                 |
| 3.3 | ドキュメント       | README または BACKUP.md に使い方追記   | docs/BACKUP.md 等                      |

---

## 6. 詳細設計

### 6.1 Rust モジュール構成

```
src-tauri/src/
├── lib.rs                 # コマンド登録: export_metadata, import_metadata
└── metadata_export.rs     # 新規: エクスポート・インポートロジック
```

※ `export_metadata.rs` と `import_metadata.rs` を分けても可。規模次第で `metadata_export.rs` にまとめる。

### 6.1b フロントエンド構成

```
src/
├── components/
│   ├── layout/
│   │   └── sidebar.tsx      # navigationItems に backup 追加
│   └── screens/
│       └── backup.tsx       # 新規: バックアップ画面
├── contexts/
│   └── navigation-context-value.ts  # Screen 型に 'backup' 追加
└── App.tsx                 # renderScreen に case 'backup' 追加
```

### 6.2 コマンド仕様

```rust
// エクスポート: 保存ダイアログを表示し、選択されたパスに ZIP を保存
#[tauri::command]
async fn export_metadata(app: AppHandle, pool: State<SqlitePool>) -> Result<ExportResult, String>;

// インポート: 開くダイアログで ZIP を選択し、マージ実行
#[tauri::command]
async fn import_metadata(app: AppHandle, pool: State<SqlitePool>, zip_path: String) -> Result<ImportResult, String>;
```

- フロントから `zip_path` を渡す場合: 先に `dialog.open()` でパスを取得し、そのパスを `import_metadata` に渡す
- または Rust 側で `dialog.open()` を呼ぶ: `tauri-plugin-dialog` を Rust から使用可能か要確認。通常はフロントで `@tauri-apps/plugin-dialog` を使い、パスを取得してから `import_metadata` に渡す形が簡潔。

### 6.3 JSON スキーマ（参考）

各テーブルはそのまま行を配列で出力。

```json
// images.json
[
  {"id": 1, "item_name_normalized": "xxx", "file_name": "xxx.jpg", "created_at": "..."},
  ...
]
```

- `id` はインポート時には使用しない（INSERT OR IGNORE で新規採番）
- UNIQUE キー（item_name_normalized, sender_address+parser_type, raw_name）で重複判定

### 6.4 権限（capabilities）

`tauri-plugin-dialog` 追加時、`default.json` に以下を追加:

```json
"permissions": [
  ...,
  "dialog:default",
  "dialog:allow-save",
  "dialog:allow-open"
]
```

---

## 7. リスク・注意点

| 項目                 | 対応                                                                                               |
| -------------------- | -------------------------------------------------------------------------------------------------- |
| 大容量 ZIP           | 画像が多い場合はストリーミング or チャンク処理を検討。初版はメモリに載る想定で実装                 |
| マイグレーション差分 | エクスポート時のスキーマとインポート先のスキーマが異なる場合、互換性チェックを manifest に持たせる |
| E2E モック           | エクスポート/インポートコマンドを E2E 時にモックするか、実際に一時 ZIP を作成するか要検討          |

---

## 8. 実装順序（推奨）

1. **Task 1.1, 1.2**: プラグイン・crate 追加
2. **Task 1.3**: エクスポートコマンド実装（単体テスト付き）
3. **Task 1.4**: インポートコマンド実装（単体テスト付き）
4. **Task 2.1**: ナビゲーション追加（Screen 型・サイドバー・App.tsx）
5. **Task 2.2–2.4**: バックアップ画面作成とボタン実装
6. **Task 3.1–3.3**: テスト・ドキュメント

---

## 9. 検証チェックリスト

- [ ] サイドバーに「データのバックアップ」が表示され、クリックで新規画面に遷移する
- [ ] エクスポート後、ZIP を展開して images / shop_settings / product_master の JSON と画像が含まれる
- [ ] 空の DB にインポートして、データが投入される
- [ ] 既にデータがある DB にインポートして、既存データが上書きされずマージされる（INSERT OR IGNORE）
- [ ] バックアップ画面から「データのバックアップ」「データの復元」が動作する
- [ ] E2E モード時にエラーにならない（モック or スキップ）
