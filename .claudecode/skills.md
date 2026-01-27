# PAA Project Development Skills

このドキュメントは、Claude Codeが本プロジェクトでタスクを実行する際の標準作業手順（SOP）を定義したものです。

## Skill: Standard_Development_Cycle

Issueの割り当てからPRのマージまでを、品質を担保しながら完遂するメインスキルです。

### 1. 準備 (Analysis & Branching)

- `gh issue view [ID]` で要件を分析する。
- `git checkout main` および `git pull` を行い、最新状態から `feature/issue-[ID]` ブランチを作成する。

### 2. 実装 (Implementation)

- Rust (src-tauri) または React (src) のコードを実装する。
- 外部API（Gmail等）やDBに依存する機能の場合、必ず `mockall` 等を用いたモック化を行い、ユニットテストを可能にする。

### 3. 検証 (Verification)

- **Rustテスト**: `cargo test` が全てパスすること。
- **カバレッジ**: `cargo llvm-cov` を実行し、新規ロジック（特にパーサー）がカバーされていることを確認する。
- **フロントテスト**: `npm run test:ui` (vitest) を実行する。
- **静的解析**: `cargo clippy` および `npm run lint` で警告がないことを確認する。

### 4. プルリクエスト (Pull Request)

- `gh pr create` でPRを作成する。
- タイトル: `feat: [概要] (closes #[ID])`
- 本文: 実施した変更内容、テスト結果、およびカバレッジのサマリーを記載する。

### 5. クリーンアップ (Cleanup)

- CIのパスを確認後、`gh pr merge --merge --delete-branch` でマージとブランチ削除を行う。
- ローカルブランチを `git branch -d` で削除する。

---

## Skill: Reactive_Development_Loop (Revised)

テスト失敗、CIエラー、および **GitHub Copilot/Reviewer からの指摘**に対して、合格するまで自律的に修正を繰り返すスキルです。

### 1. 診断 & フィードバック収集 (Diagnosis & Feedback)

- ローカル環境での `cargo test` や `npm run test:ui` のエラー出力を解析する。
- **GitHub Review の参照**: `gh pr view --comments` を実行し、Copilot や人間によるレビューコメントを抽出する。
- **CI結果の参照**: `gh run view --log` を実行し、CI（GitHub Actions）で失敗した詳細なログを読み取る。
- 指摘された「修正すべき箇所」を一覧化し、優先順位をつける。

### 2. 修正実行 (Fixing)

- 診断結果に基づき、ロジック・テストコード・静的解析エラーを修正する。
- Copilot から「この正規表現はホビーサーチの特定の形式で失敗する可能性がある」といった具体的な指摘がある場合、そのケースをカバーするユニットテストを先に追加する。

### 3. 継続的検証 & 同期 (Validation & Push)

- 修正後、関連するテストをローカルで再実行する。
- ローカルで合格したら、`git commit --amend` または `push` を行い、PRの状態を更新する。
- 更新後、再び `gh run watch` 等で CI の通過を確認する。

### 4. 完了報告

- 全ての指摘（Copilot のコメント含む）に対応し、CI がグリーンになったらユーザーに報告する。
- 指摘に対してどう修正したかのサマリーを添える。
