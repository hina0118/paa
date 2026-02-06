# PR #73 マージ前計画

**PR**: [#73 feat: メタデータのインポート/エクスポート機能 (issue #40)](https://github.com/hina0118/paa/pull/73)  
**ブランチ**: `issue40` → `main`  
**作成日**: 2026-02-07  
**CI ステータス**: pending

---

## 1. PR 概要

| 項目           | 内容                                                      |
| -------------- | --------------------------------------------------------- |
| **タイトル**   | feat: メタデータのインポート/エクスポート機能 (issue #40) |
| **変更量**     | +1,519 / -9 行、14 ファイル                               |
| **コミット数** | 10                                                        |
| **マージ可否** | mergeable（unstable）                                     |

### 主な変更

- **エクスポート**: images, shop_settings, product_master テーブルと画像ファイルを ZIP 形式でエクスポート
- **インポート**: ZIP からデータをインポート（INSERT OR IGNORE でマージ）
- **新規画面「データのバックアップ」**: サイドバーに追加
- **依存関係**: tauri-plugin-dialog, zip crate
- **ドキュメント**: docs/BACKUP.md, docs/plans/issue-40-metadata-import-export-plan.md

---

## 2. レビューコメント状況

### 解決済み（13件）

| 指摘                         | 対応内容                                              |
| ---------------------------- | ----------------------------------------------------- |
| Zip Slip（インポート）       | `enclosed_name()` で正規化、`images/` 直下のみ許可    |
| 画像メモリ消費（インポート） | `MAX_IMAGE_ENTRY_SIZE` でサイズ上限                   |
| manifest.json 検証           | バージョンチェック追加                                |
| JsonImageRow コメント        | 「id を含むがインポート時は未使用」に修正             |
| 単体テスト                   | export/import/INSERT OR IGNORE のテスト追加           |
| Sidebar 型                   | `Screen` を import して `Extract<Screen, ...>` に統一 |
| docs Zip Slip 記述           | 計画書に検証手順を追記                                |
| エクスポート file_name 検証  | `is_safe_file_name()` で検証、不正はスキップ          |
| インポート file_name 検証    | `is_safe_file_name()` で検証、不正は `None`           |
| read_zip_entry サイズ上限    | `MAX_JSON_ENTRY_SIZE` で 10MB 上限                    |
| エクスポート画像サイズ       | `fs::metadata` で事前チェック、超過はスキップ         |
| React.ComponentType          | `import type { ComponentType } from 'react'` に変更   |
| 重複 import                  | テスト内の `ZipArchive` 重複を解消                    |

### 未解決（3件）→ 対応済み

| #   | 優先度 | ファイル           | 指摘内容                                                                     | 対応状況                                                        |
| --- | ------ | ------------------ | ---------------------------------------------------------------------------- | --------------------------------------------------------------- |
| 1   | P1     | metadata_export.rs | エクスポート時、`is_safe_file_name` 不一致やサイズ超過で画像を黙ってスキップ | ✅ `ExportResult.images_skipped` 追加、フロントでスキップ時警告 |
| 2   | P1     | metadata_export.rs | インポート時、ZIP 内の `images/*` をすべてコピー。DoS の可能性               | ✅ `images.json` の `file_name` のみコピー対象に限定            |
| 3   | P1     | metadata_export.rs | DB への INSERT と画像コピーがトランザクション無し                            | ✅ DB 操作をトランザクションで囲み、commit 後に画像コピー       |

---

## 3. マージ前対応タスク

### Phase A: 未解決レビュー対応（推奨）

#### A-1. ExportResult にスキップ件数を追加

**ファイル**: `src-tauri/src/metadata_export.rs`, `src/components/screens/backup.tsx`

- `ExportResult` に `images_skipped: usize` を追加
- エクスポートループで `is_safe_file_name` 不一致・サイズ超過・ファイル存在しないの件数をカウント
- フロントで `images_skipped > 0` の場合にトーストで警告表示

#### A-2. インポート時の画像コピー対象を images.json に限定

**ファイル**: `src-tauri/src/metadata_export.rs`

- 現状: ZIP 内の `images/*` をすべて走査してコピー
- 変更: `images.json` の `file_name` を `is_safe_file_name` でフィルタした集合を `HashSet` で保持
- ZIP 内の `images/<file_name>` エントリのうち、`file_name` がその集合に含まれるもののみコピー

#### A-3. DB 操作をトランザクションで囲む

**ファイル**: `src-tauri/src/metadata_export.rs`

- `import_metadata_from_reader` 内で `pool.begin()` でトランザクション開始
- images, shop_settings, product_master の INSERT をすべて transaction 内で実行
- 成功時 `commit()`、失敗時 `rollback()`（自動）
- 画像ファイルコピーは DB commit 後に実行（DB 失敗時は何もコピーしない）

---

### Phase B: 検証・CI

| タスク         | コマンド            | 確認内容               |
| -------------- | ------------------- | ---------------------- |
| Rust テスト    | `cargo test -p paa` | 単体テストが通る       |
| フロントビルド | `npm run build`     | ビルドエラーなし       |
| Lint           | `npm run lint`      | ESLint エラーなし      |
| E2E            | `npm run test:e2e`  | ナビゲーション等が通る |

---

### Phase C: マージ後

- Copilot レビュー再依頼（未解決コメントを対応済みとして返答）
- 必要に応じて `docs/pr73-review-action-plan.md` を更新し、完了済みとしてマーク

---

## 4. 実施順序（推奨）

1. **A-2** インポート画像コピー対象の限定（セキュリティ・DoS 対策）
2. **A-3** DB トランザクション化（整合性）
3. **A-1** エクスポートスキップ件数通知（UX）
4. **Phase B** 検証・CI
5. プッシュ → レビュー返答 → マージ

---

## 5. 参考リンク

- [PR #73](https://github.com/hina0118/paa/pull/73)
- [Issue #40](https://github.com/hina0118/paa/issues/40)
- [docs/pr73-review-action-plan.md](./pr73-review-action-plan.md) — 既存の対応計画
