# PR #75 マージ計画

**PR**: [#75 feat: タスクトレイメニューからバッチ処理を直接実行（メインウィンドウ不要）](https://github.com/hina0118/paa/pull/75)  
**作成日**: 2026-02-07  
**更新日**: 2026-02-07  
**ブランチ**: `tasktray` → `main`  
**ステータス**: Open / mergeable_state: unstable（CI 待ち）

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

| ファイル                          | 変更内容      |
| --------------------------------- | ------------- |
| `src-tauri/src/batch_commands.rs` | 新規（670行） |
| `src-tauri/src/lib.rs`            | +119 / -600   |
| `docs/pr75-plan.md`               | 本ファイル    |
| `docs/pr75-review-action-plan.md` | 対応計画      |

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

### 2.2 未解決（3件）→ 本計画で対応

GitHub レビューで **IsResolved: false** のまま残っている 3 件を以下の残タスクで対応します。

---

## 3. 残タスク対応計画

### Task A: P1 - run_sync_task の「既に実行中」時に set_error を呼ばない

**ファイル**: `src-tauri/src/batch_commands.rs`  
**行**: 52-66 付近

**指摘**: `sync_state.try_start()` が false の場合に `set_error("Sync is already in progress")` を設定すると、単に二重起動しただけでも `last_error` が残り、同期完了後に `get_sync_status` が `error` を返し続ける。

**修正案**: 「既に実行中」の場合は `set_error` を呼ばず、イベント通知のみ行う。`SyncState::try_start()` は `bool` を返すため、現状は「既に実行中」と「Mutex poison 等」を区別できない。最小限の対応として、`set_error` の呼び出しを削除する。

```rust
if !sync_state.try_start() {
    log::warn!("Sync is already in progress");
    let message = "Sync is already in progress".to_string();
    // set_error は呼ばない（二重起動の場合は last_error を汚染しない）
    let error_event = BatchProgressEvent::error(
        GMAIL_SYNC_TASK_NAME, 0, 0, 0, 0, message,
    );
    let _ = app.emit(GMAIL_SYNC_EVENT_NAME, error_event);
    return;
}
```

**将来的な改善**: `try_start()` を `Result<(), String>` に変更し、Mutex poison 時のみ `set_error` する（別 PR で検討）。

---

### Task B: P1 - run_batch_parse_task の「既に実行中」時に set_error を呼ばない

**ファイル**: `src-tauri/src/batch_commands.rs`  
**行**: 279-292 付近

**指摘**: `parse_state.start()` の失敗（特に "Parse is already running"）で `set_error` を設定すると、処理が進行中なだけでも `last_error` が残り、処理完了後に `get_parse_status` が `error` になり続ける。

**修正案**: 「Parse is already running」の場合は `set_error` を呼ばずイベント通知のみ。Mutex poison 等の真のエラー時のみ `set_error` を呼ぶ。

```rust
if let Err(e) = parse_state.start() {
    let msg = e.to_string();
    if msg.contains("Parse is already running") {
        log::warn!("Parse already running, skip starting new parse: {}", msg);
        let error_event = BatchProgressEvent::error(
            EMAIL_PARSE_TASK_NAME, 0, 0, 0, 0,
            format!("Parse already running: {}", msg),
        );
        let _ = app.emit(EMAIL_PARSE_EVENT_NAME, error_event);
    } else {
        log::error!("Failed to start parse: {}", msg);
        parse_state.set_error(&e);
        let error_event = BatchProgressEvent::error(
            EMAIL_PARSE_TASK_NAME, 0, 0, 0, 0,
            format!("Parse error: {}", msg),
        );
        let _ = app.emit(EMAIL_PARSE_EVENT_NAME, error_event);
    }
    return;
}
```

---

### Task C: P2 - トレイメニュー経由の batch_size 取得失敗時にログを出力

**ファイル**: `src-tauri/src/lib.rs`  
**行**: 741-748 付近

**指摘**: `app_config_dir()` / `config::load()` の失敗を `.ok()` 連鎖で握りつぶしており、default(100) にフォールバックした理由がログに残らない。

**修正案**: `match` で分岐し、失敗時は `log::warn!` で原因をログ出力する。

```rust
let batch_size = match app.path().app_config_dir() {
    Ok(dir) => match config::load(&dir) {
        Ok(c) => batch_commands::clamp_batch_size(c.parse.batch_size, 100),
        Err(e) => {
            log::warn!(
                "Failed to load config from {:?}: {}. Falling back to default batch_size=100",
                dir, e
            );
            100
        }
    },
    Err(e) => {
        log::warn!(
            "Failed to get app_config_dir: {}. Falling back to default batch_size=100",
            e
        );
        100
    }
};
```

---

## 4. 実行順序

| 順序 | Task   | 内容                                                                     |
| ---- | ------ | ------------------------------------------------------------------------ |
| 1    | Task A | batch_commands.rs: sync try_start 失敗時に set_error を削除              |
| 2    | Task B | batch_commands.rs: parse start 失敗時、「既に実行中」は set_error しない |
| 3    | Task C | lib.rs: tray_parse の batch_size 取得失敗時に log::warn を追加           |
| 4    | -      | `cargo test` / `cargo clippy` / `cargo fmt` で確認                       |
| 5    | -      | プッシュ後、CI が green になることを確認                                 |
| 6    | -      | マージ                                                                   |

---

## 5. マージ前チェックリスト

- [x] Task A, B, C の修正を適用
- [ ] ローカルで `cargo test` が成功
- [ ] ローカルで `cargo clippy` に警告なし
- [ ] CI が green
- [ ] 手動でトレイメニューから各バッチ処理を実行して動作確認

---

## 6. 参考リンク

- [PR #75](https://github.com/hina0118/paa/pull/75)
- [レビューコメント対応計画 (pr75-review-action-plan.md)](./pr75-review-action-plan.md)
- [Copilot レビュー](https://github.com/hina0118/paa/pull/75#pullrequestreview-3766235554)
