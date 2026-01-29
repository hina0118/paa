# Issue #47 画像管理方式移行（BLOB → ローカルファイル保存）実装計画

> **For Claude:** 実装時は superpowers:executing-plans を使用してタスク単位で進めること。

**Goal:** `images` テーブルの `image_data` (BLOB) を廃止し、アプリデータディレクトリ直下の `images/` にファイルとして保存する方式へ移行する。DB には `file_name` (TEXT) のみ保持する。

**Phase 1（今回）:** 画像の取り込み処理は未実装のため、**DB マイグレーションのみ**実施する。`image_data` 削除・`file_name` 追加でスキーマを先行変更する。

**Phase 2（将来・画像取り込み実装時）:** `images/` ディレクトリ・保存ユーティリティ、asset プロトコル / CSP、フロントの `convertFileSrc` 表示、保存コマンド、バックアップドキュメント（Task 2〜6）。

**Tech Stack:** Tauri 2, SQLite (tauri-plugin-sql / sqlx)。Phase 2 で React, `@tauri-apps/api`, Rust std::fs 等を利用。

---

## 現状サマリ

| 項目         | 状態                                                                                                                     |
| ------------ | ------------------------------------------------------------------------------------------------------------------------ |
| DB           | `images` テーブル: `id`, `item_id`, `image_data` (BLOB), `created_at`。`005_create_images_table.sql`。                   |
| Rust         | 画像の保存・読み出しロジックは未実装。`app_data_dir` は `lib.rs` setup で使用（DB 配置）。                               |
| フロント     | `TableViewer` で `images` を表示（汎用テーブルビュー）。`ItemImage` 型に `imageData?: string`。`convertFileSrc` 未使用。 |
| バックアップ | 手動エクスポート機能は未実装。                                                                                           |

---

## タスク一覧

**今回実施するのは Task 1 のみ。** Task 2〜6 は画像取り込み実装時に実施する（後述「Phase 2」）。

### Task 1: DB マイグレーション（`image_data` 削除・`file_name` 追加）【今回】

**目的:** `images` テーブルから `image_data` を削除し、`file_name` (TEXT) を追加する。

**Files:**

- Create: `src-tauri/migrations/020_images_blob_to_file_storage.sql`
- Modify: `src-tauri/src/lib.rs`（マイグレーション配列に 020 を追加）

**手順:**

1. **マイグレーション SQL を作成**
   - `image_data` を `file_name` に変更するだけ。`ALTER TABLE` で `ADD COLUMN file_name` → `DROP COLUMN image_data`（SQLite 3.35.0+）。

```sql
-- 020: images の image_data (BLOB) を file_name (TEXT) に変更
ALTER TABLE images ADD COLUMN file_name TEXT;
ALTER TABLE images DROP COLUMN image_data;
```

2. **`lib.rs` のマイグレーション配列に 020 を追加**
   - `Migration { version: 20, description: "images: blob to file_name", sql: include_str!(...), kind: MigrationKind::Up }` を追加。

3. **動作確認**
   - `npm run tauri dev` で起動し、マイグレーションが通ることを確認。
   - `Images` テーブル表示でエラーにならないこと、`file_name` カラムが存在することを確認。

4. **コミット**
   - `git add src-tauri/migrations/020_*.sql src-tauri/src/lib.rs`
   - `git commit -m "chore(db): migrate images from BLOB to file_name (issue #47)"`

---

## Phase 2（将来・画像取り込み実装時）

以下は画像の取り込み・表示を実装する際に実施する。**今回はスキップする。**

### Task 2: `images/` ディレクトリの確保と Rust 保存ユーティリティ

**目的:** 起動時に `app_data_dir/images/` を作成し、画像バイト列をファイル保存して `file_name` を返すユーティリティを用意する。

**Files:**

- Create: `src-tauri/src/image_storage.rs`（例）
- Modify: `src-tauri/src/lib.rs`（`images` ディレクトリ作成、`image_storage` 利用の受け入れ準備）
- 既存のパース・Gmail 同期などで画像保存を行う場合は、ここで用意した API を呼ぶ想定。

**手順:**

1. **`image_storage` モジュールの追加**
   - `ensure_images_dir(app: &AppHandle) -> Result<PathBuf, Error>`: `app_data_dir` を取得し、`images/` を作成。`images` の `PathBuf` を返す。
   - `save_image(app: &AppHandle, item_id: i64, bytes: &[u8], extension: &str) -> Result<String, Error>`:
     - `ensure_images_dir` で `images/` を確保。
     - ファイル名は `{sha256_truncated}_{item_id}.{ext}` などユニークな形で生成（例: `sha2` の最初の 16 文字 + `_` + `item_id` + `.jpg`）。
     - `std::fs::write` で保存し、保存したファイル名（先頭に `images/` を含めない）を返す。
   - 拡張子は呼び出し元から渡す（`.jpg` / `.png` 等）。未指定時は `.bin` などデフォルトを定義。

2. **起動時処理**
   - `lib.rs` の `setup` 内で、DB 初期化後に `ensure_images_dir` を呼ぶ。失敗時は `log::error` しつつ起動は継続するか、`expect` するかはプロジェクト方針に合わせる。

3. **単体テスト（任意）**
   - 一時ディレクトリを使い `ensure_images_dir` と `save_image` の動作を検証。

4. **コミット**
   - `git add src-tauri/src/image_storage.rs src-tauri/src/lib.rs`
   - `git commit -m "feat(storage): add images dir and save_image utility (issue #47)"`

---

### Task 3: Tauri アセットプロトコルと CSP の設定

**目的:** フロントから `convertFileSrc` で `app_data_dir/images/` 内のファイルを表示できるようにする。

**Files:**

- Modify: `src-tauri/tauri.conf.json`

**手順:**

1. **`assetProtocol` を有効化し、`images` 用スコープを追加**
   - `app.security.assetProtocol`: `enable: true`、`scope` にアプリデータディレクトリの `images` 以下を許可するパターンを追加。
   - 例（Windows）: `$APPDATA` 配下のアプリフォルダを用いている場合、`["$APPDATA/jp.github.hina0118.paa/images/*"]` 等。他 OS は Tauri の `app_data_dir` に対応する変数・パスを調べる。
   - 複数プラットフォーム対応の場合、利用可能な変数（`$APPDATA`, `$CONFIG` 等）で適宜追加。

2. **CSP で `img-src` に asset を許可**
   - 現在 `app.security.csp` は `null`。`convertFileSrc` 利用時は `img-src` に `asset:` と `http://asset.localhost` を含める必要がある。
   - 例: `"csp": "default-src 'self'; img-src 'self' asset: http://asset.localhost"`（既存の `connect-src` 等があるなら合わせて維持）。

3. **動作確認**
   - Task 4 以降で画像表示を実装したうえで、`convertFileSrc` により `images/` 内の画像が表示されることを確認。

4. **コミット**
   - `git add src-tauri/tauri.conf.json`
   - `git commit -m "config(tauri): enable asset protocol for images dir (issue #47)"`

---

### Task 4: フロントエンドの型・表示対応（`file_name` と `convertFileSrc`）

**目的:** `ItemImage` を `file_name` ベースにし、`images` テーブルや注文詳細などで `file_name` から画像を表示する。

**Files:**

- Modify: `src/lib/types.ts`（`ItemImage` の `imageData` → `fileName`）
- Modify: `src/components/tables/table-viewer.tsx`（`images` の `file_name` カラムで画像表示）
- 注文詳細等で `ItemImage` を表示しているコンポーネントがあれば、同様に `fileName` + `convertFileSrc` に切り替える。

**手順:**

1. **型定義の更新**
   - `ItemImage`: `imageData?: string` を削除し、`fileName?: string` を追加。

2. **画像パス取得ヘルパー**
   - `appDataDir()` と `join` で `images` ディレクトリを解決し、`join(appDataDir, 'images', fileName)` でフルパスを組み立てる。
   - 実行環境が Tauri かどうかで分岐する場合は、Tauri 時のみ `convertFileSrc` を使用する。

3. **`TableViewer` の拡張**
   - `tableName === 'images'` かつカラムが `file_name` のとき、値をファイル名として扱う。
   - `fileName` が存在すれば、上記ヘルパーでパスを組み立て → `convertFileSrc` で URL 化 → `<img src={...} alt="..." />` で表示（サムネイル等）。それ以外は従来どおりテキスト表示。

4. **その他参照箇所**
   - `OrderWithDetails` / `OrderWithSources` の `items[].image` を使っている UI があれば、`image.fileName` を同様に `convertFileSrc` で表示するよう変更。

5. **テスト**
   - `TableViewer` の `images` 表示、既存のフロント単体テスト・E2E で画像表示に依存するものがあれば実行。

6. **コミット**
   - `git add src/lib/types.ts src/components/tables/table-viewer.tsx` 等
   - `git commit -m "feat(ui): show images via file_name and convertFileSrc (issue #47)"`

---

### Task 5: 画像保存の Tauri コマンド（任意・必要に応じて）

**目的:** 画像のインポートやダウンロード機能を実装する場合、Rust 側で `save_image` を呼び出し、DB に `file_name` を挿入するコマンドを用意する。

**Files:**

- Modify: `src-tauri/src/lib.rs`（`invoke_handler` に `save_product_image` 等を追加）
- Modify: `src-tauri/src/image_storage.rs`（既存の `save_image` を利用）

**手順:**

1. **コマンドの追加**
   - 例: `save_product_image(item_id: i64, bytes: Vec<u8>, extension: Option<String>) -> Result<String, String>`
   - `save_image` でファイル保存 → 返却された `file_name` を使って `images` に `INSERT` または `UPDATE`（1 item 1 image の制約に合わせる）。
   - `file_name` をフロントに返す。

2. **フロント**
   - インポート/ダウンロード完了時に `invoke('save_product_image', { ... })` を呼ぶ。

3. **既存機能**
   - 現状、画像のダウンロード・インポートは未実装のため、このタスクは「今後それらを実装するときの土台」としても可。その場合はコマンド追加のみで完了とし、呼び出しは別 Issue でもよい。

4. **コミット**
   - `git commit -m "feat(tauri): add save_product_image command (issue #47)"`

---

### Task 6: 手動バックアップ対象に `images/` を含める

**目的:** 手動エクスポート・バックアップの際に `images/` を含めるようドキュメント化する。

**Files:**

- Create or Modify: `docs/BACKUP.md` または `README.md` / 既存の運用ドキュメント

**手順:**

1. **ドキュメントの追加・更新**
   - バックアップ対象として以下を含める旨を明記する:
     - `paa_data.db`（DB ファイル。パスは `app_data_dir` 直下。）
     - `images/` ディレクトリ（`app_data_dir/images/`）
   - 可能なら、`app_data_dir` の場所（OS 別）も簡単に記載する。

2. **コミット**
   - `git add docs/BACKUP.md` 等
   - `git commit -m "docs: include images/ in backup instructions (issue #47)"`

---

## 検証チェックリスト（Phase 1・今回）

- [ ] マイグレーション 020 適用後、`images` に `file_name` があり `image_data` がない
- [ ] `npm run tauri dev` で起動し、マイグレーションが通る
- [ ] Images テーブル表示でエラーにならず、`file_name` カラムが存在する

---

## 注意事項

- **Phase 1（今回）:** マイグレーションのみ。型・フロント・asset プロトコル・Rust 保存ロジックは一切変更しない。`TableViewer` はそのまま `file_name` 列をテキスト表示する。
- **既存 BLOB データ:** 移行しない。必要なら別タスクで BLOB → ファイル出力のマイグレーションスクリプトを検討する。
- **DB パス:** 現行は `paa_data.db`。Issue の `paa_database.db` は記載ゆれと解釈し、`paa_data.db` を維持する。
- **Phase 2 以降:** 画像表示や `save_product_image` 等を追加した場合、E2E やモックの更新が必要になる可能性がある。

---

## 実行オプション（Phase 1）

**今回の対象は Task 1（マイグレーション）のみ。**

1. **Subagent-Driven（このセッション）** — Task 1 を実装し、検証チェックリストで確認後にコミット。
2. **別セッションで実行** — 新規セッションで executing-plans を使い、Task 1 のみ実行。

Phase 2（Task 2〜6）は画像取り込み実装時に本計画を参照して実施する。
