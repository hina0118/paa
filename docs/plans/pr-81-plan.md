# PR #81 対応計画

**作成日**: 2026-02-10  
**PR**: [#81 DMM DMM通販メールの最終状態対応（分割・まとめ・発送）](https://github.com/hina0118/paa/pull/81)  
**ブランチ**: `feature/dmm-cancel` → `main`  
**未解決レビュー**: 0件（対応済み）

---

## PR 概要

**タイトル**: DMM DMM通販メールの最終状態対応（分割・まとめ・発送）  
**ブランチ**: `feature/dmm-cancel` → `main`  
**状態**: Open  
**CI**: pending（コミット `d438596`）  
**mergeable_state**: clean

**変更内容**:

DMM通販のメールフロー全体を最終状態ベースで扱えるように対応。ご注文分割完了・ご注文まとめ完了・発送完了・注文番号変更メールを組み合わせて、DB 上の注文を現実世界の最終状態に寄せる。

1. **DMM分割完了メール（ご注文分割完了のお知らせ）**
   - dmm_split_complete パーサーを追加（parse_multi で複数注文をパース）
   - apply_split_first_order: 先頭注文を「元注文」として扱い、既存注文があれば items を差し替え、なければ新規作成

2. **DMMまとめ完了メール（ご注文まとめ完了のお知らせ）**
   - dmm_merge_complete パーサーを追加
   - apply_consolidation: 複数注文を1注文に統合

3. **DMM発送完了メール（ご注文商品を発送いたしました）**
   - dmm_send パーサーを追加（商品・金額・配送業者・伝票番号・受取人名を抽出）
   - apply_send_and_replace_items: 既存注文があれば items/金額で上書き、deliveries を shipped に更新

4. **注文番号変更メール**
   - 旧注文が見つからない場合でも、新注文番号で新規作成するフォールバック
   - order_date は注文番号変更メールの internal_date で初期化

5. **件名フィルタの緩和**
   - shop_settings.subject_filters を複数パターン対応に変更
   - 「DMM通販：」が付かない件名もパース対象に

**主な変更ファイル**:

- `src-tauri/src/parsers/dmm_*.rs`（dmm_split_complete, dmm_merge_complete, dmm_send 等）
- `src-tauri/src/parsers/mod.rs`
- `src-tauri/src/repository.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/migrations/007_add_dmm_split_complete_shop_settings.sql` ～ `010_update_dmm_subject_filters.sql`

---

## レビューコメント一覧

### Resolved（対応済み）

| #   | 優先度 | 指摘                                                                         | 状態                                                      |
| --- | ------ | ---------------------------------------------------------------------------- | --------------------------------------------------------- |
| 1   | P0     | get_body_for_parse() が body_html を常に優先し、plain パーサーに生HTMLが渡る | Resolved（HTMLでパースできるように調整済み）              |
| 2   | P0     | マイグレーション006が一覧に含まれていない                                    | Resolved（将来的に001へ集約予定、いったんこのままでよい） |
| 3   | P1     | 010の件名フィルタ更新で mono 側（info@mono.dmm.com）が更新されない           | Resolved                                                  |

### 対応済み（本対応で解消）

| #   | 優先度 | 指摘                                       | 対応内容                                                                                               |
| --- | ------ | ------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| 4   | P0     | SQLite trigram tokenize のバージョン保証   | 起動時に `SELECT sqlite_version()` でチェックし、3.43 未満の場合は明確なエラーメッセージで panic       |
| 5   | P1     | LOWER(order_number) によるインデックス阻害 | マイグレーション 011 で `idx_orders_order_number_shop_domain_nocase` を追加し、`COLLATE NOCASE` に統一 |

---

## 対応方針

### 推奨: 未対応指摘を解消してからマージ

- **P0 指摘（#4）**: SQLite trigram の環境保証を明確にする
- **P1 指摘（#5）**: order_number のケース揺れ対応とインデックス整合

工数は中程度、1PRで完結可能。

---

## 実施タスク

### 1. SQLite trigram のバージョン保証（P0）

| 項目 | 内容                                                                                                                                           |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| 対象 | `src-tauri/Cargo.toml` / `001_init.sql`                                                                                                        |
| 内容 | `tokenize='trigram'` を使用する場合、SQLite 3.43+ を保証する必要がある                                                                         |
| 案A  | sqlx の `sqlite-bundled` または `sqlite-bundled-libsqlite3-sys` を有効化して bundled SQLite を使用                                             |
| 案B  | 環境の SQLite バージョンを起動時にチェックし、3.43未満の場合は trigram FTS をスキップするフォールバック（既存 FTS テーブルにフォールバック等） |
| 案C  | trigram を当面オプション機能とし、001_init.sql では trigram を使わず、006 相当を別マイグレーションで条件付き適用                               |

**確認事項**: 現在 sqlx の feature 設定で bundled SQLite が有効かどうかを確認する。

### 2. LOWER(order_number) とインデックス整合（P1）

| 項目 | 内容                                                                                       |
| ---- | ------------------------------------------------------------------------------------------ |
| 対象 | `src-tauri/src/repository.rs` およびインデックス定義                                       |
| 問題 | `LOWER(order_number) = LOWER(?)` では通常の B-tree インデックスが使われにくい              |
| 案A  | `COLLATE NOCASE` を使用: `order_number = ? COLLATE NOCASE`（SQLite の NOCASE collation）   |
| 案B  | 式インデックスを追加: `CREATE INDEX ... ON orders(LOWER(order_number), shop_domain)`       |
| 案C  | アプリ側で order_number を正規化（小文字化等）して保存・検索し、インデックスをそのまま利用 |

**確認事項**: DMM の order_number が大文字小文字混在する実データがあるか、既存の保存形式との整合性を確認する。

---

## 実行フロー

1. Task 1（P0）: SQLite trigram の対応方針を決定し、bundled 有効化またはフォールバックを実装
2. Task 2（P1）: order_number の検索条件とインデックスを整合させる
3. `cargo fmt` / `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo test --all --all-features`
5. コミット・プッシュ
6. CI 通過を確認
7. Copilot レビュー再依頼

---

## マージ前チェックリスト

- [x] Task 1: SQLite trigram のバージョン保証（起動時チェック）
- [x] Task 2: LOWER(order_number) とインデックスの整合（COLLATE NOCASE + 011）
- [x] `cargo fmt` / `cargo clippy` 成功
- [x] `cargo test --all --all-features` 成功
- [ ] CI 通過
- [ ] Copilot レビュー再依頼

---

## 備考

- 010 の subject_filters 更新で mono 側（info@mono.dmm.com）を含める修正提案がレビューにあった。Resolved 扱いだが、未マージの場合は両送信元を対象にした `WHERE sender_address IN ('info@mail.dmm.com', 'info@mono.dmm.com')` の適用を確認すること。
- DMM のメールフローは、手続き完了 → 分割/まとめ → 発送 → キャンセル/番号変更 の順で多様なパターンがあり、今回の対応で最終状態に寄せる処理が網羅されている。
