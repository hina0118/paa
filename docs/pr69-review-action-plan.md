# PR #69 レビューコメント対応計画

**PR**: [#69 fix: Gmail messages_get scope + migration preload](https://github.com/hina0118/paa/pull/69)  
**作成日**: 2026-02-05  
**ブランチ**: `feature/gmail-scope-migration-preload`  
**CI ステータス**: pending

---

## 概要

PR #69 は以下を実装しています：

1. **Gmail messages_get スコープ修正** — `Scope::Readonly` を追加（"Missing access token for authorization" エラー対策）
2. **Migration preload** — `tauri.conf.json` に `plugins.sql.preload` を追加、DB パスを `app_config_dir` に統一
3. **BatchRunner 移行** — `sync_gmail_incremental` を削除し `start_sync` / `BatchRunner` に統一
4. **filter_new_message_ids** — メモリ最適化のため `get_existing_message_ids` を置換

---

## 未対応レビューコメント一覧

| #   | 優先度            | ファイル                      | 行   | 指摘内容                                                                                           | 対応方針                                            |
| --- | ----------------- | ----------------------------- | ---- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| 1   | **P0: Critical**  | `src/lib/database.ts`         | 123  | DB は appConfigDir、画像は appDataDir のままで不整合の可能性                                       | 方針を明確化し、必要ならドキュメント化              |
| 2   | **P1: Important** | `src-tauri/src/repository.rs` | 1091 | `temp_filter_ids` を明示的に DROP する                                                             | tx.commit() 前に DROP を追加                        |
| 3   | **P2: Nitpick**   | `src-tauri/tauri.conf.json`   | 24   | preload の `sqlite:paa_data.db` が app_config_dir を正しく解決するか確認、ドキュメントコメント追加 | tauri-plugin-sql 仕様を確認し、必要ならコメント追加 |
| 4   | **P2: Nitpick**   | `src-tauri/src/repository.rs` | 2081 | `filter_new_message_ids` の CHUNK_SIZE 超えテスト（1000件、2000件等）の追加                        | テストケース追加                                    |

**解決済み**: messages_get の `add_scope`（動作確認のため維持）、repository.rs プライバシーコメント（IsOutdated）

---

## 対応計画

### Phase 1: P0 — DB/画像パスの方針明確化

**指摘**: データベースは `appConfigDir` に移動されたが、画像は `appDataDir` のまま。不整合により画像アクセス失敗の可能性がある。

**現状整理**:

| データ種別         | ディレクトリ     | 使用箇所                                                                                   |
| ------------------ | ---------------- | ------------------------------------------------------------------------------------------ |
| DB (`paa_data.db`) | `app_config_dir` | `database.ts`, `lib.rs`, tauri-plugin-sql preload                                          |
| 画像 (`images/`)   | `app_data_dir`   | `useImageUrl.ts`, `lib.rs` (search_product_images), `assetProtocol` (`$APPDATA/images/**`) |

**分析**:

- 画像まわり（`useImageUrl`、`assetProtocol`、`lib.rs` の `app_data_dir.join("images")`）はすべて `appDataDir` で一貫している
- DB は `appConfigDir` に統一されている
- 意図的な設計: 設定系データ（DB）とユーザーデータ（画像）を分離している

**対応方針**:

- **オプション A（推奨）**: 現状維持を明文化
  - DB: `appConfigDir`、画像: `appDataDir` の設計をコメントやドキュメントで明記
  - レビューコメントへ「意図的な設計であり、画像まわりは appDataDir で一貫している」旨を返答
- **オプション B**: 画像も `appConfigDir` へ移行
  - `useImageUrl.ts`、`lib.rs`、`tauri.conf.json` の `assetProtocol` をすべて `appConfigDir` に変更
  - 設計変更とユーザー移行の影響が大きいため、今回は行わない想定

**推奨**: オプション A。必要に応じて `docs/` に「データ配置方針」を追記する。

---

### Phase 2: P1 — temp_filter_ids の明示的 DROP

**ファイル**: `src-tauri/src/repository.rs`  
**場所**: `filter_new_message_ids` 内、`tx.commit()` の直前

**修正例**:

```rust
// tx.commit() の前に追加
sqlx::query("DROP TABLE IF EXISTS temp_filter_ids")
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to drop temp table: {e}"))?;

tx.commit()
    .await
    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
```

SQLite の TEMP テーブルは接続単位で分離されているが、意図を明確にするため明示的な DROP を追加する。

---

### Phase 3: P2 — preload パス解決の確認とドキュメント

**ファイル**: `src-tauri/tauri.conf.json`  
**指摘**: `sqlite:paa_data.db` の相対パスが `app_config_dir` を正しく解決するか不明

**対応**:

1. tauri-plugin-sql の preload が `app_config_dir` を基準とするか仕様を確認
2. 必要であれば `tauri.conf.json` や `lib.rs` の DB 初期化周りにコメントを追加
   - 例: 「preload の `sqlite:paa_data.db` は app_config_dir 基準で解決される」

---

### Phase 4: P2 — filter_new_message_ids のテスト拡張

**ファイル**: `src-tauri/src/repository.rs`  
**対象**: `filter_new_message_ids` のテスト

**追加するテストケース**:

1. CHUNK_SIZE (900) ちょうどの件数
2. CHUNK_SIZE を超える件数（例: 1000件、2000件）
3. すべて既存 ID の場合（空を返す）
4. すべて新規 ID の場合（全て返す）

既存の `test_email_repository_save_and_get` 内の `filter_new_message_ids` テストを拡張するか、別テスト関数を追加する。

---

## 実装順序

| 順  | 項目                                      | 優先度    | 工数目安 |
| --- | ----------------------------------------- | --------- | -------- |
| 1   | P0: DB/画像パス方針の明確化とレビュー返答 | Critical  | 15分     |
| 2   | P1: temp_filter_ids の DROP 追加          | Important | 5分      |
| 3   | P2: preload のドキュメントコメント        | Nitpick   | 10分     |
| 4   | P2: filter_new_message_ids テスト拡張     | Nitpick   | 20分     |

---

## 完了条件

- [x] P0: レビューコメントへ方針説明を返答（必要ならドキュメント更新） — database.ts に設計意図のコメント追加済み
- [x] P1: `filter_new_message_ids` 内で `temp_filter_ids` を明示的に DROP
- [x] P2: preload のパス解決についてコメントを追加（lib.rs）
- [x] P2: `filter_new_message_ids` のエッジケーステストを追加（test_filter_new_message_ids_chunk_boundaries）
- [ ] CI 成功
- [ ] 全レビュースレッド解決済み

---

## 参考リンク

- [PR #69](https://github.com/hina0118/paa/pull/69)
- [Issue #68 対応計画](./issue-68-plan.md) — filter_new_message_ids の設計元
