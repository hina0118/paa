# PR #73 レビューコメント対応計画

**PR**: [#73 feat: メタデータのインポート/エクスポート機能 (issue #40)](https://github.com/hina0118/paa/pull/73)  
**作成日**: 2026-02-07  
**ブランチ**: `issue40`  
**CI ステータス**: pending

---

## 概要

PR #73 は Issue #40 のメタデータインポート/エクスポート機能を実装しています。

### 主な変更

- **エクスポート**: images, shop_settings, product_master テーブルと画像ファイルを ZIP 形式でエクスポート
- **インポート**: ZIP からデータをインポート（INSERT OR IGNORE でマージ）
- **新規画面「データのバックアップ」**: サイドバーに追加
- **tauri-plugin-dialog**, **zip crate** の導入
- **docs/BACKUP.md**: 使い方ドキュメント

---

## 未対応レビューコメント一覧

| #   | 優先度            | ファイル             | 行  | 指摘内容                                                                                                                            | 対応方針                                             |
| --- | ----------------- | -------------------- | --- | ----------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| 1   | **P0: Critical**  | `metadata_export.rs` | 181 | エクスポート時、`file_name` を未検証で `images_dir.join(file_name)` に使用。パストラバーサルでアプリ外のファイルを ZIP に取り込める | `file_name` が単一ファイル名であることを検証         |
| 2   | **P0: Critical**  | `metadata_export.rs` | 271 | インポート時、`images.json` の `file_name` を未検証で DB に保存。将来のエクスポートでパストラバーサルに繋がる                       | インポート時点で `file_name` をサニタイズ/検証       |
| 3   | **P0: Critical**  | `sidebar.tsx`        | 34  | `React.ComponentType` を参照しているが `React` を import していない。ビルドエラーの可能性                                           | `import type { ComponentType } from 'react'` で置換  |
| 4   | **P1: Important** | `metadata_export.rs` | 381 | `read_zip_entry` が JSON をサイズ上限なしで `read_to_string`。巨大 ZIP で DoS の可能性                                              | `entry.size()` で上限を設ける                        |
| 5   | **P1: Important** | `metadata_export.rs` | 185 | エクスポート時、画像を `fs::read` で丸ごとメモリ読み込み。大量画像でメモリ消費                                                      | サイズ上限を設ける、またはストリーミング             |
| 6   | **P1: Important** | `metadata_export.rs` | 47  | 単体テストの追加（export→ZIP検証、import→件数検証、INSERT OR IGNORE）                                                               | 既存テストでカバー済みの旨を確認し、不足があれば追加 |

**解決済み（6件）**: Zip Slip 対策（import）、画像メモリ消費（import）、manifest.json 検証、コメント修正、Sidebar 型、docs の Zip Slip 記述

---

## 対応計画

### Phase 1: P0 — セキュリティ・ビルド修正（最優先）

#### 1-1. エクスポート時の `file_name` 検証

**ファイル**: `src-tauri/src/metadata_export.rs`  
**場所**: 171–186 行付近（画像エクスポートループ内）

**現状**: `file_name_opt` をそのまま `images_dir.join(file_name)` と `images/<file_name>` に使用。

**修正方針**:

- `file_name` が「単一のファイル名（`/`、`\`、`..` を含まない）」であることを検証するヘルパー関数を追加
- 不正な `file_name` はスキップ（またはエラー返却）

```rust
/// file_name が安全な単一ファイル名か検証（パストラバーサル対策）
fn is_safe_file_name(file_name: &str) -> bool {
    !file_name.is_empty()
        && !file_name.contains('/')
        && !file_name.contains('\\')
        && !file_name.contains("..")
        && file_name == Path::new(file_name).file_name().and_then(|n| n.to_str()).unwrap_or("")
}
```

- エクスポートループ内で `if !is_safe_file_name(file_name) { continue; }` を追加

---

#### 1-2. インポート時の `file_name` 検証

**ファイル**: `src-tauri/src/metadata_export.rs`  
**場所**: 255–272 行付近（images の INSERT ループ）

**現状**: `row.2`（file_name）をそのまま DB に保存。

**修正方針**:

- `is_safe_file_name` で検証し、不正な場合はその行をスキップ（INSERT しない）
- または `file_name` を `None` にして INSERT（スキップの方が一貫性あり）

```rust
for row in &images_rows {
    let file_name = row.2.as_ref().filter(|fn_| is_safe_file_name(fn_));
    let result = sqlx::query(...)
        .bind(&row.1)
        .bind(file_name)  // 不正な場合は None でスキップ
        ...
}
```

- レビュー指摘は「拒否」を推奨しているため、不正な `file_name` の行は INSERT しない（`file_name` を `None` にするか、その行ごとスキップ）

---

#### 1-3. sidebar.tsx の `React.ComponentType` 修正

**ファイル**: `src/components/layout/sidebar.tsx`  
**場所**: 32–34 行

**現状**:

```ts
icon: React.ComponentType<{ className?: string }>;
```

**修正**:

```ts
import type { ComponentType } from 'react';
// ...
icon: ComponentType<{ className?: string }>;
```

---

### Phase 2: P1 — メモリ・DoS 対策

#### 2-1. `read_zip_entry` にサイズ上限を追加

**ファイル**: `src-tauri/src/metadata_export.rs`  
**場所**: `read_zip_entry` 関数（366–378 行）

**修正方針**:

- JSON エントリ用のサイズ上限定数を追加（例: `MAX_JSON_ENTRY_SIZE: u64 = 10 * 1024 * 1024` = 10MB）
- `entry.size()` を確認し、上限超過なら `Err` を返す
- `read_to_string` の前にチェック

```rust
const MAX_JSON_ENTRY_SIZE: u64 = 10 * 1024 * 1024; // 10MB

fn read_zip_entry<R: Read + Seek>(...) -> Result<String, String> {
    let mut entry = archive.by_name(name)?;
    if entry.size() > MAX_JSON_ENTRY_SIZE {
        return Err(format!("{} exceeds size limit ({} bytes)", name, MAX_JSON_ENTRY_SIZE));
    }
    // ...
}
```

---

#### 2-2. エクスポート時の画像サイズ上限

**ファイル**: `src-tauri/src/metadata_export.rs`  
**場所**: 174–186 行（画像読み込み・ZIP 書き込み）

**現状**: `fs::read(&src)` で丸ごと読み込み。`MAX_IMAGE_ENTRY_SIZE` はインポート側のみで使用。

**修正方針**:

- エクスポート時も `fs::metadata(&src).map(|m| m.len())` でサイズを確認
- `MAX_IMAGE_ENTRY_SIZE` 超過の画像はスキップ（既存定数を流用）

```rust
let metadata = fs::metadata(&src).ok();
if metadata.map(|m| m.len() > MAX_IMAGE_ENTRY_SIZE).unwrap_or(true) {
    continue; // サイズ不明 or 超過はスキップ
}
let data = fs::read(&src)?;
```

---

### Phase 3: P1 — 単体テストの確認

**指摘**: export→ZIP 検証、import→件数/画像コピー検証、INSERT OR IGNORE のテストを追加。

**現状**:

- `test_export_import_roundtrip`: 既存 DB → export → 空 DB へ import → 件数検証 ✓
- `test_export_zip_contents`: ZIP 内に manifest.json, images.json 等が含まれることを検証 ✓
- `test_import_insert_or_ignore_duplicate`: 同一 DB への再 import で重複が無視されることを検証 ✓
- `test_import_with_image_files`: 画像ファイルのコピー検証 ✓
- `test_import_rejects_wrong_manifest_version`: manifest バージョン拒否 ✓

**対応方針**:

- 上記で指摘の (1)(2)(3) は既にカバー済み
- レビューコメントに「該当テストが追加済みである」旨を返答し、解決済みとする
- 必要に応じて `file_name` 検証（不正な file_name をスキップする挙動）のテストを追加

---

## 実施順序

1. **Phase 1-3**: sidebar.tsx の `ComponentType` 修正（即時）
2. **Phase 1-1**: エクスポート時の `file_name` 検証
3. **Phase 1-2**: インポート時の `file_name` 検証
4. **Phase 2-1**: `read_zip_entry` のサイズ上限
5. **Phase 2-2**: エクスポート時の画像サイズ上限
6. **Phase 3**: テスト確認、必要なら追加
7. **CI 実行**: `cargo test`, `npm run lint`, `npm run build` 等で確認
8. **プッシュ後**: Copilot へレビュー依頼

---

## 参考

- [Zip Slip 脆弱性](https://snyk.io/research/zip-slip-vulnerability)
- `.cursorrules` の Review Priority System（P0 は即座に指摘・修正）
