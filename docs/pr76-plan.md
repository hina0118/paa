# PR #76 マージ計画

**PR**: [#76 feat: ホビーサーチ注文キャンセルメール対応](https://github.com/hina0118/paa/pull/76)  
**作成日**: 2026-02-07  
**更新日**: 2026-02-08  
**ブランチ**: `cancel-mail` → `main`  
**ステータス**: Open / mergeable_state: clean / CI: pending  
**未解決レビュー**: 0件（Task F/G/H 対応済み）

---

## 1. 概要

ホビーサーチの注文キャンセル完了通知メールをパースし、既存注文の商品数量を減算・削除する機能を実装した変更です。

### 主な変更

| 項目                            | 内容                                                                               |
| ------------------------------- | ---------------------------------------------------------------------------------- |
| **hobbysearch_cancel パーサー** | `[キャンセル]` セクションから注文番号・商品名・キャンセル個数を抽出                |
| **apply_cancel**                | 注文検索 → 商品マッチング → 数量減算/削除 → order_emails 紐付け                    |
| **process_batch 改善**          | confirm/change を即時 save_order（同一バッチ内で cancel が先に来ても注文参照可能） |
| **商品名マッチング**            | 完全一致 → 包含 → item_name_normalized（括弧除去版も試行）                         |
| **strip_bracketed_content**     | 【】[]（）() で囲まれた部分を除去して比較                                          |

### 変更ファイル

| ファイル                                       | 変更内容                  |
| ---------------------------------------------- | ------------------------- |
| `src-tauri/src/parsers/hobbysearch_cancel.rs`  | 新規（108行）             |
| `src-tauri/src/parsers/email_parse_task.rs`    | キャンセル分岐・即時 save |
| `src-tauri/src/parsers/mod.rs`                 | キャンセル分岐追加        |
| `src-tauri/src/repository.rs`                  | apply_cancel 追加         |
| `src-tauri/src/logic/email_parser.rs`          | hobbysearch_cancel 登録   |
| `src-tauri/migrations/001_init.sql`            | shop_settings 追加        |
| `src-tauri/src/batch_commands.rs`              | デバッグログ追加          |
| `docs/plans/hobbysearch-cancel-mail-design.md` | 設計書（新規）            |
| `.cursorrules`                                 | graphQL 記述修正          |

### 設計書

`docs/plans/hobbysearch-cancel-mail-design.md` を参照

---

## 2. Copilot レビュー指摘

### 対応済み（追加対応 2026-02-08）

- **P0**: batch_parse_emails の confirm/change 即時 save 対応
- **P1**: 注文検索フォールバックの shop_domain 条件（ shop_domain 渡時はフォールバックしない、フォールバック時は `shop_domain IS NULL OR ''` に限定）
- **P1**: cancel_quantity <= 0 の検証を追加
- **P2**: strip_bracketed_content のループを 1 回の replace_all に簡略化
- **P2**: order_emails 紐付け・重複チェックの統合テスト追加

### 元の指摘（4件）

### P1: マイグレーション（001_init.sql）→ 対応不要

**ファイル**: `src-tauri/migrations/001_init.sql`  
**指摘**: `001_init.sql` は既存DBには再適用されないため、この行を追加しても既存ユーザー環境には `hobbysearch_cancel` の shop_settings が入らない。  
**判断**: リリース前のため対応不要。001 に追加するだけで十分。

### P1: キャンセルが先に来るケース

**ファイル**: `src-tauri/src/parsers/email_parse_task.rs`  
**指摘**: 同一 run 内で「キャンセルメールが先に処理され、対応する confirm/change が後に来る」ケースでは、cancel 側が `apply_cancel` 失敗のまま再試行されず、このメールは次回 run まで未パース扱いのまま残る。`internal_date` の並びが崩れることを想定するなら、(a) cancel を一旦キューして全注文の `save_order` 後に再試行する、または (b) cancel 失敗時に後続入力に該当注文が作られたら再適用する、といった二段階処理が必要。

### P1: apply_cancel のテスト不足

**ファイル**: `src-tauri/src/repository.rs`  
**指摘**: `apply_cancel` は数量減算/削除・order_emails 紐付けまで行う重要ロジックだが、`SqliteOrderRepository` の既存テスト群に対してキャンセル適用の DB 状態を検証するテストが追加されていない。リグレッション防止のため、最小でも「1件減算」「0で削除」「商品不一致」「注文不一致」をカバーする統合テストを追加する必要がある。

### P2: ログの個人情報リスク

**ファイル**: `src-tauri/src/batch_commands.rs`  
**指摘**: メール件名（subject）を `info` レベルでログ出力すると、デバッグビルドではログバッファ/UI に件名が残り得る（内容によっては個人情報になりやすい）。トラブルシュート目的なら `debug` レベルに落とす、もしくは subject を省略/マスクする運用に寄せるのが安全。

### P2: コメントと実装の不一致 → 対応済み

**ファイル**: `src-tauri/src/repository.rs` 行 669  
**指摘**: コメント「order_number + shop_domain、見つからねば order_number のみで再検索」と実装が一致していない。実際のフォールバックは `shop_domain IS NULL OR ''` に限定されている。  
**対応**: コメントを「shop_domain 未設定の注文のみで再検索」に修正済み。

---

## 3. 対応計画

### Task A: マイグレーション 002 → 不要

**判断**: リリース前のため、001 への追加のみで十分。既存ユーザー向けの 002 マイグレーションは不要。

---

### Task B: P1 - キャンセルが先に来るケースへの対応

**目的**: 同一バッチ内で cancel が confirm/change より先に来た場合でも、キャンセルメールが「未パース」のまま残らないようにする。

**選択肢**:

| 案  | 内容                                                                                                 | メリット          | デメリット                                               |
| --- | ---------------------------------------------------------------------------------------------------- | ----------------- | -------------------------------------------------------- |
| A   | cancel 失敗時も `Ok(EmailParseOutput)` を返し、`parsed_emails` としてマーク。次回 run で再試行される | 実装が軽い        | 次回 run までキャンセルが反映されない                    |
| B   | cancel を一旦キューし、全メールの save_order 後にキャンセルを再試行                                  | 同一 run 内で完結 | 実装が複雑                                               |
| C   | cancel 失敗時は `Err` を返し、未パース扱いのまま残す（現状）                                         | 既存のまま        | 次回 run まで残るが、設計書 6.3 の「スキップ」方針と整合 |

**推奨**: 設計書 6.3 では「稀にキャンセルが先に来た場合: 該当 order が存在しなければ、キャンセルはスキップ（ログ出力）」と記載。実装上は `Err` を返すと「未パース」のまま残り、次回 run で再試行される。この挙動は設計書と整合しているため、**現状維持 + 設計書への追記**で対応可能。  
ただし Copilot の指摘「再試行されない」は誤解があり、未パースメールは次回 run で再度パース対象になる。この点を PR コメントで説明するか、`Err` ではなく `Ok` で返して `parsed_emails` にマークし「パースは成功したがキャンセル適用は失敗」として扱う案 A を検討する。

**修正案（案 A）**: Copilot の suggestion に従い、`apply_cancel` 失敗時も `Ok(EmailParseOutput { cancel_applied: false })` を返す。これにより該当メールは「パース済み」としてマークされ、`parsed_emails` に記録される。次回 run では未パースメールから除外されるが、キャンセル適用は失敗したまま。  
→ この場合、キャンセルが反映されない注文が残るリスクがある。`Err` で返して「未パース」のままにしておけば、次回 run で再度パースが試行され、その時点で confirm が先に来ていれば成功する。  
→ **結論**: `Err` のまま（現状維持）が正しい。設計書 6.3 の記載を明確にし、PR コメントで Copilot に「次回 run で再試行される」旨を返信する。

---

### Task C: P1 - apply_cancel の統合テスト追加

**目的**: リグレッション防止のため、以下のシナリオをカバーするテストを追加する。

| テストケース | 内容                                                             |
| ------------ | ---------------------------------------------------------------- |
| 1件減算      | 数量 2 の商品に cancel_quantity=1 を適用 → 1 になる              |
| 0で削除      | 数量 1 の商品に cancel_quantity=1 を適用 → item が DELETE される |
| 商品不一致   | 存在しない商品名で apply_cancel → Err                            |
| 注文不一致   | 存在しない order_number で apply_cancel → Err                    |

**ファイル**: `src-tauri/src/repository.rs` の `#[cfg(test)]` 内、または `tests/` ディレクトリに新規テストファイル。

**実装方針**: `SqliteOrderRepository` を使用し、事前に orders / items を INSERT した状態で `apply_cancel` を呼び、結果の DB 状態を検証する。

---

### Task D: P2 - ログレベルの見直し

**目的**: メール件名（subject）の info ログによる個人情報リスクを軽減する。

**対応**: `batch_commands.rs` の該当ログを `log::info!` から `log::debug!` に変更。または subject を省略/マスクする。

---

### Task E: P2 - コメント修正 → 対応済み

**目的**: 注文検索ロジックのコメントを実装に合わせる。

**ファイル**: `src-tauri/src/repository.rs` 行 669

**修正案**:

```rust
// 修正前
// 1. 既存の注文を検索（order_number + shop_domain、見つからねば order_number のみで再検索）

// 修正後
// 1. 既存の注文を検索（order_number + shop_domain、見つからねば shop_domain 未設定の注文のみで再検索）
```

---

### Task F: P1 - get_parser に hobbysearch_cancel の扱いがない（未解決）

**ファイル**: `src-tauri/src/parsers/mod.rs` 行 35 付近  
**指摘**: `hobbysearch_cancel` を `is_valid_parser_type` に追加しているが、`get_parser()` は `hobbysearch_cancel` を返さない。`parse_email` や `parse_and_save_email` 経由ではキャンセルメールが「Unknown parser type」またはスキップされる。

**状況**: キャンセルメールは `OrderInfo` ではなく `CancelInfo` を返すため、`EmailParser` トレイト（`parse() -> OrderInfo`）にそのまま乗せられない。設計上、キャンセル処理は `batch_parse_emails` 専用。

**対応案**:

| 案  | 内容                                                                 | メリット                          | デメリット                                                        |
| --- | -------------------------------------------------------------------- | --------------------------------- | ----------------------------------------------------------------- |
| A   | `get_candidate_parsers` で hobbysearch_cancel を除外                 | 単一メール parse で候補に出さない | parse_and_save_email はキャンセルメールを処理できない（設計通り） |
| B   | `get_parser` に hobbysearch_cancel 用ダミーを追加（常に Err を返す） | 一貫性あり                        | 呼び出し元で「Unknown」ではなくパース失敗になる                   |
| C   | 現状維持（batch のみ対応）                                           | 設計書・実装と整合                | Copilot 指摘の「一貫性」は満たさない                              |

**推奨**: 案 A。キャンセルメールはバッチ専用とし、`get_candidate_parsers` が単一メール用の候補を返す際に hobbysearch_cancel を除外する。`parse_and_save_email` は元来「注文作成/更新」用であり、キャンセルはバッチで処理する設計と整合。

---

### Task G: P2 - フォールバッククエリの同条件再検索（未解決）

**ファイル**: `src-tauri/src/repository.rs` 行 687 付近  
**指摘**: 最初の検索が `COALESCE(shop_domain, '') = COALESCE(?, '')` のため、`shop_domain` が None/空のときは既に「NULL/空の注文」を検索できている。フォールバックは実質同条件の再検索になり、注文未存在時は余計なクエリになる。

**対応案**: フォールバックを削除してロジックを単純化するか、分岐でクエリを切り替える。ただし、shop_domain が渡っている場合に「渡っていない注文」をフォールバックで探す意図があるなら、現状の分岐は妥当。レビューで意図を確認し、不要ならフォールバック削除。

---

### Task H: P1 - apply_cancel 失敗時の failed_count / log::error（未解決）

**ファイル**: `src-tauri/src/parsers/mod.rs` 行 472 付近  
**指摘**: `apply_cancel` 失敗時（注文未作成など）でも `failed_count += 1` と `log::error!` を行う。次イテレーションで再試行され得るため、最終的に成功しても失敗として二重カウント／エラーログ過多になり得る。

**対応案**: リトライ前提の失敗は `log::warn!` に落とす。`failed_count` は「最終的に未処理で残った」場合のみ加算する形に変更する。

---

## 4. マージ前チェックリスト

### 対応済み

- [x] Task A: マイグレーション 002 → 不要（リリース前のため）
- [x] Task C: apply_cancel の統合テスト追加
- [x] Task D: ログレベルの見直し
- [x] Task B: 設計書 6.3 の記載確認（現状維持の場合は PR コメントで説明）
- [x] P1: 注文検索フォールバックの shop_domain 条件を修正
- [x] P1: cancel_quantity の検証を追加
- [x] P2: strip_bracketed_content の簡略化
- [x] P2: order_emails 紐付けの統合テスト追加
- [x] P0: batch_parse_emails の confirm/change 即時 save 対応
- [x] `cargo test` 成功
- [x] Task E: 注文検索コメントの修正（repository.rs 行 669）

### 対応済み（2026-02-08 実装）

- [x] Task F: get_candidate_parsers で hobbysearch_cancel を除外
- [x] Task G: フォールバッククエリを削除（同条件の再検索だったため簡略化）
- [x] Task H: apply_cancel 失敗時 log::error → log::warn、failed_count 加算を廃止

### マージ条件

- [ ] CI 成功

---

## 5. マージまでの残作業

| 優先度 | 項目        | 見積 |
| ------ | ----------- | ---- |
| 1      | CI 成功確認 | 自動 |

---

## 6. 備考

- **mergeable_state: clean**: コンフリクトなし。CI が pending の場合は結果を待つ。
- **デバッグログ**: `batch_commands.rs` および `parsers/mod.rs` に追加された `[parse]` `[batch]` `[DEBUG]` ログは、マージ前に `debug` レベルに統一するか、本番ビルドでは出さないようにすることを検討する。
- **Task F/G/H**: いずれも対応済み。
