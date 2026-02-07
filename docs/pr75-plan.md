# PR #75 マージ計画

**PR**: [#75 feat: タスクトレイメニューからバッチ処理を直接実行（メインウィンドウ不要）](https://github.com/hina0118/paa/pull/75)  
**作成日**: 2026-02-07  
**ブランチ**: `tasktray` → `main`  
**ステータス**: Open / mergeable / CI: clean

---

## 1. 概要

タスクトレイメニューから各バッチ処理（Gmail同期・メールパース・商品名解析）をメインウィンドウを開かずに直接実行できるようにする変更です。

### 主な変更

| 項目                          | 内容                                                                                        |
| ----------------------------- | ------------------------------------------------------------------------------------------- |
| **batch_commands モジュール** | 新設。`run_sync_task` / `run_batch_parse_task` / `run_product_name_parse_task` として共通化 |
| **タスクトレイメニュー**      | 「バッチ処理」サブメニュー追加（Gmail同期・メールパース・商品名解析）                       |
| **トレイからの実行**          | `app.try_state()` 経由でバックエンドから直接バッチ実行（フロントエンド不要）                |
| **既存コマンド**              | `start_sync` 等を `batch_commands` を呼ぶ薄いラッパーにリファクタ                           |

### 変更ファイル

| ファイル                          | 変更内容         |
| --------------------------------- | ---------------- |
| `src-tauri/src/batch_commands.rs` | 新規（666行）    |
| `src-tauri/src/lib.rs`            | +119 / -600      |
| `docs/pr75-review-action-plan.md` | 新規（対応計画） |

---

## 2. 対応状況

### 2.1 対応済み（20件）

Copilot レビューで指摘された以下の項目は対応済みです（`pr75-review-action-plan.md` 参照）:

- finish() の誤呼び出し削除（P0）
- batch_size のバリデーション（P0/P1）
- app_config_dir 失敗時の処理（P1）
- clamp_batch_size の Doc コメント（P2）
- try_start() の Err をそのまま返す（P1）
- その他 15 件

### 2.2 未解決（2件）→ 対応済み ✅

GitHub レビューで **IsResolved: false** のまま残っていた 2 件は、本計画の実行により対応済みです。

---

## 3. 残タスク対応計画

### Task A: P1 - run_batch_parse_task の parse_state.start() 失敗時に set_error を呼ぶ

**ファイル**: `src-tauri/src/batch_commands.rs`  
**行**: 280-286 付近

**指摘**: `parse_state.start()` が失敗した場合（Mutex poison や既に実行中など）、`BatchProgressEvent` を emit するだけで `ParseState.last_error` が更新されないため、`get_parse_status` が "idle" のままになり得る。

**修正案**:

```rust
if let Err(e) = parse_state.start() {
    log::error!("Failed to start parse: {}", e);
    parse_state.set_error(&e);  // ← 追加
    let error_event = BatchProgressEvent::error(
        EMAIL_PARSE_TASK_NAME,
        0, 0, 0, 0,
        format!("Parse error: {}", e),
    );
    let _ = app.emit(EMAIL_PARSE_EVENT_NAME, error_event);
    return;
}
```

---

### Task B: P1 - run_sync_task の try_start() 失敗時に set_error を呼ぶ

**ファイル**: `src-tauri/src/batch_commands.rs`  
**行**: 52-65 付近

**指摘**: `SyncState::try_start()` は「既に実行中」だけでなく Mutex poison 等のロック取得失敗でも `false` を返すが、ここでは常に "Sync is already in progress" を返しており原因切り分けができない。また `last_error` も更新されないため `get_sync_status` が idle のままになり得る。

**現状**: `SyncState::try_start()` は `bool` を返すため、失敗理由を区別できない。

**修正案（最小限）**: 失敗時に `sync_state.set_error()` を呼ぶ。メッセージは現状のままでも、少なくとも `get_sync_status` で error を返せるようになる。

```rust
if !sync_state.try_start() {
    log::warn!("Sync is already in progress");
    let message = "Sync is already in progress".to_string();
    sync_state.set_error(&message);  // ← 追加
    let error_event = BatchProgressEvent::error(
        GMAIL_SYNC_TASK_NAME, 0, 0, 0, 0, message,
    );
    let _ = app.emit(GMAIL_SYNC_EVENT_NAME, error_event);
    return;
}
```

**将来的な改善案**: `SyncState::try_start()` を `Result<(), String>` に変更し、Mutex poison 時は `Err(e.to_string())` を返す（別 PR で検討可）。

---

## 4. 実行順序

| 順序 | Task   | 内容                                                                |
| ---- | ------ | ------------------------------------------------------------------- |
| 1    | Task A | batch_commands.rs: parse_state.start() 失敗時に set_error を追加    |
| 2    | Task B | batch_commands.rs: sync_state.try_start() 失敗時に set_error を追加 |
| 3    | -      | `cargo test` / `cargo clippy` / `cargo fmt` で確認                  |
| 4    | -      | プッシュ後、Copilot へ再レビュー依頼（任意）                        |
| 5    | -      | マージ                                                              |

---

## 5. マージ前チェックリスト

- [x] Task A, B の修正を適用
- [ ] ローカルで `cargo test` が成功
- [ ] ローカルで `cargo clippy` に警告なし
- [ ] CI が green
- [ ] 手動でトレイメニューから各バッチ処理を実行して動作確認

---

## 6. 参考リンク

- [PR #75](https://github.com/hina0118/paa/pull/75)
- [レビューコメント対応計画 (pr75-review-action-plan.md)](./pr75-review-action-plan.md)
- [Copilot レビュー](https://github.com/hina0118/paa/pull/75#pullrequestreview-3766235554)
