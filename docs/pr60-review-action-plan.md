# PR #60 レビューコメント対応計画

**PR**: [#60 feat(gmail): keyringでOAuth認証情報を管理](https://github.com/hina0118/paa/pull/60)  
**作成日**: 2026-02-03  
**更新日**: 2026-02-03  
**未対応コメント数**: **0件**（全3件対応済み）

---

## 概要

PR 60 に対する GitHub Copilot のレビューコメントを整理し、対応計画を作成しました。  
**全3件のコメント**が P1（重要）に分類されています。

---

## レビューコメント一覧（全3件）

| #   | ファイル                              | 行  | 優先度 | 指摘内容                                                                                                                          | 対応方針                                           |
| --- | ------------------------------------- | --- | ------ | --------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| 1   | `src-tauri/src/gmail/config.rs`       | 105 | **P1** | **削除関数の一貫性** — 他のモジュール（gemini、google_search）では削除失敗時にエラーを返しているが、Gmailではエラーを無視している | ✅ `map_err(...)?` でエラー伝播するよう修正済み    |
| 2   | `src/components/screens/settings.tsx` | 489 | **P1** | **Gmail OAuth設定のテスト不足** — SerpApi・Geminiと同様の包括的テストが必要                                                       | ✅ settings.test.tsx に Gmail OAuth テスト追加済み |
| 3   | `src/components/ui/textarea.tsx`      | 22  | **P1** | **Textareaコンポーネントのテスト不足** — button、input、checkbox と同様のテストが必要                                             | ✅ textarea.test.tsx を新規作成済み                |

---

## P1: 重要 — 3件

### #1: gmail/config.rs — 削除関数のエラーハンドリング

**指摘**: `delete_oauth_credentials()` が `entry.delete_credential()` のエラーを無視している。gemini/config.rs・google_search/config.rs では `map_err(...)?` でエラーを伝播している。

**現状コード**:

```rust
// client_idの削除
if let Ok(entry) = client_id_entry() {
    let _ = entry.delete_credential();  // エラー無視
}
// client_secretの削除
if let Ok(entry) = client_secret_entry() {
    let _ = entry.delete_credential();  // エラー無視
}
```

**参考（gemini/config.rs）**:

```rust
entry
    .delete_credential()
    .map_err(|e| format!("Failed to delete Gemini API key from secure storage: {e}"))?;
```

**対応方針**:

- `client_id_entry()?` と `client_secret_entry()?` でエントリ取得
- 両方の `delete_credential()` に `map_err(...)?` を適用
- Gmail は client_id と client_secret の2エントリがあるため、両方の削除結果をエラー伝播する

---

### #2: settings.tsx — Gmail OAuth設定のテスト追加

**指摘**: SerpApi・Gemini と同様の包括的テストが必要。

**追加すべきテストケース**:
| テスト | 内容 |
|--------|------|
| Gmail OAuth設定カードの表示 | `getByRole('heading', { name: /Gmail OAuth/ })` で表示確認 |
| JSON貼り付けでの保存成功 | `save_gmail_oauth_credentials` が正しく呼ばれ、成功メッセージ表示 |
| ファイルアップロードでの保存成功 | FileReader 経由で JSON 読み込み → 保存 → 成功メッセージ |
| 空のJSONでのバリデーションエラー | 「JSONを入力してください」表示 |
| 無効なJSON形式でのエラー | 「無効なJSON形式です」表示 |
| 削除確認ダイアログの動作 | confirm が false のとき削除しない |
| 削除成功 | confirm が true のとき `delete_gmail_oauth_credentials` 呼び出し、成功メッセージ |
| 削除失敗時のエラー表示 | `delete_gmail_oauth_credentials` が reject したときエラーメッセージ |
| 保存失敗時のエラー表示 | `save_gmail_oauth_credentials` が reject したときエラーメッセージ |

**モック設定**:

- `has_gmail_oauth_credentials`: 初期 false、削除後 false、保存後 true
- `save_gmail_oauth_credentials`: 成功/失敗のパターン
- `delete_gmail_oauth_credentials`: 成功/失敗のパターン

---

### #3: textarea.tsx — Textareaコンポーネントのテスト追加

**指摘**: input.test.tsx と同様のテストパターンで textarea.test.tsx を作成。

**追加すべきテストケース**:
| テスト | 内容 |
|--------|------|
| 基本的なレンダリング | `getByRole('textbox')` で textarea が表示される |
| classNameのマージ | `className="custom-class"` でカスタムクラスが適用される |
| ref forwarding | `React.forwardRef` で ref が正しく渡される |
| disabled状態 | `disabled` で無効化される |
| placeholder属性 | `placeholder` が正しく表示される |

**参考**: `src/components/ui/input.test.tsx` の構造に従う。

---

## 対応順序の推奨

### Phase 1: バックエンド修正（P1 #1）

1. **gmail/config.rs** — `delete_oauth_credentials()` のエラーハンドリング修正
   - `client_id_entry()?` でエントリ取得
   - `client_secret_entry()?` でエントリ取得
   - 各 `delete_credential()` に `map_err(...)?` を適用
   - 既存の `test_delete_oauth_credentials` は成功ケースのためそのまま動作する想定

### Phase 2: UIコンポーネントテスト（P1 #3）

2. **textarea.test.tsx** — 新規作成
   - input.test.tsx を参考に、Textarea 用のテストを実装
   - 依存が少なく、先行して対応可能

### Phase 3: 設定画面テスト（P1 #2）

3. **settings.test.tsx** — Gmail OAuth テスト追加
   - `describe('Gmail OAuth設定')` ブロックを追加
   - beforeEach の mockInvoke に `has_gmail_oauth_credentials` を追加
   - SerpApi テストパターンに従って各ケースを実装

---

## 技術メモ

### gmail/config.rs 修正例

```rust
/// OAuth認証情報を削除
pub fn delete_oauth_credentials() -> Result<(), String> {
    // client_idの削除
    client_id_entry()?
        .delete_credential()
        .map_err(|e| format!("Failed to delete Gmail client_id from secure storage: {e}"))?;

    // client_secretの削除
    client_secret_entry()?
        .delete_credential()
        .map_err(|e| format!("Failed to delete Gmail client_secret from secure storage: {e}"))?;

    log::info!("Gmail OAuth credentials deleted successfully from secure storage");
    Ok(())
}
```

**注意**: `client_id_entry()` と `client_secret_entry()` は keyring のエントリ取得であり、エントリが「存在しない」場合と「削除に失敗した」場合は区別される。keyring クレートの `delete_credential()` は、エントリが存在しない場合の挙動を要確認（一部 keyring 実装では「存在しない」もエラーになる可能性あり）。その場合は、gemini/google_search と同様に「削除失敗 = エラー返却」で一貫させる。

### settings.test.tsx の mockInvoke 拡張

デフォルトの mockInvoke に以下を追加:

```typescript
if (cmd === 'has_gmail_oauth_credentials') {
  return Promise.resolve(false);
}
```

Gmail テスト用の各ケースで `save_gmail_oauth_credentials`、`delete_gmail_oauth_credentials` を適宜モック。

### ファイルアップロードのテスト

`handleFileUpload` は `FileReader` を使用。テストでは `FileReader` をモックするか、`userEvent.upload()` 相当の方法でファイル入力をシミュレート。`@testing-library/user-event` の `upload` または `input.files` の直接設定を検討。

---

## 参考リンク

- [PR #60 レビューコメント](https://github.com/hina0118/paa/pull/60)
- [PR #59 レビュー対応計画](./pr59-review-action-plan.md)（フォーマット参考）
