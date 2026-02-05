# PR #70 対応計画

**作成日**: 2026-02-05  
**PR**: [#70 fix(ci): E2E・Coverage修正、商品画像即時反映、設定分離など改善](https://github.com/hina0118/paa/pull/70)

---

## 1. PR 概要

| 項目       | 内容                                       |
| ---------- | ------------------------------------------ |
| ブランチ   | `kaizen` → `main`                          |
| 状態       | Open                                       |
| マージ可否 | mergeable_state: **unstable**（CI 要確認） |
| 変更規模   | +3,734 / -4,556 行、41 ファイル            |
| コミット数 | 11                                         |

### 主な変更内容

- **CI 修正**: E2E（Sync/Parse → Batch 統合）、Coverage（settings/api-keys テスト追加、閾値 82% に一時調整）
- **機能**: 商品画像即時反映、ダッシュボード統計追加
- **リファクタ**: 設定画面分離（API Keys / Settings）、Sync/Parse を Batch に統合、マイグレーション 001 集約

---

## 2. レビュー・コメント状況

| 種別                   | 件数  | 備考                                  |
| ---------------------- | ----- | ------------------------------------- |
| 未解決レビューコメント | **0** | 対応不要                              |
| 一般コメント           | 0     | -                                     |
| Copilot レビュー       | 1 件  | 41 ファイルレビュー済み、コメントなし |

**結論**: レビュー指摘への対応は不要。

---

## 3. CI 状況

| ジョブ   | 状態   | 備考                                                      |
| -------- | ------ | --------------------------------------------------------- |
| Lint     | 要確認 | mergeable_state: unstable のため                          |
| Test     | 要確認 | -                                                         |
| Coverage | 要確認 | vitest 閾値: lines/statements 85%, functions/branches 82% |
| E2E      | 要確認 | Batch 統合後の navigation/sync/parse/tables 修正済み      |

**対応**: `/ci_plan` でローカル実行し、全チェック通過を確認する。

---

## 4. ローカル未コミット変更

以下の 3 ファイルに未コミットの変更あり（`kaizen` ブランチ）:

| ファイル                                             | 状態     |
| ---------------------------------------------------- | -------- |
| `src/components/orders/image-search-dialog.test.tsx` | Modified |
| `src/components/ui/dialog.test.tsx`                  | Modified |
| `src/components/ui/textarea.test.tsx`                | Modified |

**対応方針**:

1. 変更内容を確認し、PR に含めるか判断する
2. 含める場合: `/commit_plan` → `/push_plan` でコミット・プッシュ
3. 含めない場合: `git checkout -- <file>` で破棄

---

## 5. 推奨対応順序

### Step 1: ローカル変更の整理

- [ ] 3 ファイルの差分を確認
- [ ] PR に含めるか決定
- [ ] 含める場合はコミット・プッシュ

### Step 2: CI 確認

- [ ] `/ci_plan` を実行
- [ ] lint / format / test / coverage が通ることを確認
- [ ] 失敗があれば修正

### Step 3: マージ準備

- [ ] CI が緑になることを GitHub 上で確認
- [ ] 必要に応じて Copilot に再レビュー依頼
- [ ] マージ実行

---

## 6. 関連ドキュメント

- [docs/ci-fix-action-plan.md](./ci-fix-action-plan.md) — CI 修正の詳細計画（Phase 1・2 完了済み）

---

## 7. 次のアクション

1. **即時**: ローカル 3 ファイルの変更を確認・整理
2. **即時**: `/ci_plan` で CI 相当のチェックを実行
3. **CI 通過後**: `/push_plan` でプッシュ（未コミット変更をコミットする場合）
