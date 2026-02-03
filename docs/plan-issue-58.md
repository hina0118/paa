# Issue #58 対応計画: Gmail認証情報のkeyring管理

## 概要

Gmail APIの認証情報（`client_secret.json`の`client_id`と`client_secret`）をOSのセキュアストレージ（keyring）で管理するようリファクタリングする。

**注意**: `token.json`の扱いは変更しない。既存JSONファイルからのマイグレーションは不要。

## 現状分析

### 現在の認証フロー

1. `src-tauri/src/gmail.rs`の`GmailClient::new()`で認証を実行
2. `client_secret.json`ファイルを`app_data_dir`から読み込み
3. `oauth2::read_application_secret()`でファイルを解析
4. `InstalledFlowAuthenticator`で認証フローを実行

### 既存のkeyring実装パターン

- `src-tauri/src/gemini/config.rs`: Gemini APIキーのkeyring管理
- `src-tauri/src/google_search/config.rs`: SerpApi APIキーのkeyring管理
- パターン: `Entry::new("paa-{service}", "{key-name}")`

## 実装計画

### Phase 1: バックエンド（Rust）

#### 1.1 Gmail OAuth設定管理モジュールの作成

**ファイル**: `src-tauri/src/gmail/config.rs`（新規作成）

```rust
// keyringエントリ名
- service: "paa-gmail-oauth"
- client_id_key: "gmail-client-id"
- client_secret_key: "gmail-client-secret"

// 関数
- has_oauth_credentials() -> bool
- load_oauth_credentials() -> Result<(String, String), String>
- save_oauth_credentials(client_id: &str, client_secret: &str) -> Result<(), String>
- delete_oauth_credentials() -> Result<(), String>
```

#### 1.2 JSON入力からの保存機能

**ファイル**: `src-tauri/src/gmail/config.rs`

```rust
// client_secret.jsonの形式からclient_idとclient_secretを抽出して保存
- save_oauth_credentials_from_json(json_content: &str) -> Result<(), String>
```

JSON形式（Google Cloud Consoleからダウンロード）:

```json
{
  "installed": {
    "client_id": "xxxxx.apps.googleusercontent.com",
    "client_secret": "xxxxx",
    ...
  }
}
```

#### 1.3 gmail.rsの認証フロー修正

**ファイル**: `src-tauri/src/gmail.rs`

```rust
// 変更点
- read_application_secret()の代わりにkeyringから認証情報を読み込み
- ApplicationSecretを手動で構築
- ファイル存在チェックを削除（keyring有無チェックに変更）
```

#### 1.4 Tauriコマンドの追加

**ファイル**: `src-tauri/src/lib.rs`

```rust
// 新規コマンド
#[tauri::command]
async fn has_gmail_oauth_credentials() -> bool

#[tauri::command]
async fn save_gmail_oauth_credentials(json_content: String) -> Result<(), String>

#[tauri::command]
async fn delete_gmail_oauth_credentials() -> Result<(), String>
```

### Phase 2: フロントエンド（React/TypeScript）

#### 2.1 設定画面の更新

**ファイル**: `src/components/screens/settings.tsx`

Gmail OAuth設定セクションを追加:

- 設定状態の表示（設定済み/未設定）
- JSON入力方法:
  - **テキストエリア**: client_secret.jsonの内容を貼り付け
  - **ファイルアップロード**: client_secret.jsonをアップロード
- 保存・削除ボタン
- セキュリティ説明文（OSのセキュアストレージに保存される旨）

#### 2.2 UI設計

```
┌─────────────────────────────────────────────────────────────┐
│ Gmail OAuth設定                                              │
├─────────────────────────────────────────────────────────────┤
│ Google Cloud Consoleから取得したOAuth認証情報を設定します      │
│ （OSのセキュアストレージに保存）                               │
│                                                             │
│ ステータス: [設定済み ✓] or [未設定]                          │
│                                                             │
│ 設定方法を選択:                                              │
│ ○ JSONを貼り付け  ○ ファイルをアップロード                   │
│                                                             │
│ [貼り付けの場合]                                             │
│ ┌───────────────────────────────────────────────────────┐ │
│ │ client_secret.jsonの内容を貼り付け...                   │ │
│ │                                                       │ │
│ └───────────────────────────────────────────────────────┘ │
│                                                             │
│ [アップロードの場合]                                         │
│ [ファイルを選択] client_secret.json                          │
│                                                             │
│ [保存] [削除]                                               │
└─────────────────────────────────────────────────────────────┘
```

### Phase 3: テスト

#### 3.1 バックエンドテスト

**ファイル**: `src-tauri/src/gmail/config.rs`内のテストモジュール

- `test_has_oauth_credentials_returns_false_when_empty`
- `test_has_oauth_credentials_returns_true_when_set`
- `test_save_and_load_oauth_credentials`
- `test_save_oauth_credentials_from_json`
- `test_delete_oauth_credentials`
- `test_save_oauth_credentials_from_invalid_json`

#### 3.2 フロントエンドテスト

**ファイル**: `src/components/screens/settings.test.tsx`（既存に追加）

- Gmail OAuth設定の表示テスト
- JSON貼り付け保存テスト
- ファイルアップロードテスト
- 削除機能テスト

## ファイル変更一覧

### 新規作成

1. `src-tauri/src/gmail/mod.rs` - gmailモジュール定義
2. `src-tauri/src/gmail/config.rs` - OAuth設定管理

### 変更

1. `src-tauri/src/gmail.rs` → `src-tauri/src/gmail/client.rs`にリネーム（モジュール化）
2. `src-tauri/src/lib.rs` - 新規コマンド追加
3. `src/components/screens/settings.tsx` - Gmail OAuth設定セクション追加
4. `src/components/screens/settings.test.tsx` - テスト追加

## 依存関係

- 既存の`keyring` crateを使用（Cargo.tomlに追加済み）
- `serde_json`でJSON解析

## セキュリティ考慮事項

1. client_id/client_secretはログに出力しない
2. UIでは入力後にクリア
3. セキュアストレージの使用をユーザーに明示

## 実装順序

1. `src-tauri/src/gmail/config.rs`の作成
2. `src-tauri/src/gmail.rs`のモジュール化と認証フロー修正
3. `src-tauri/src/lib.rs`にTauriコマンド追加
4. フロントエンドの設定画面更新
5. テストの実装
6. 動作確認

## 見積もり工数

- Phase 1（バックエンド）: 主要作業
- Phase 2（フロントエンド）: 中程度
- Phase 3（テスト）: 中程度

---

## 実装完了状況

### 完了した作業

1. **Gmail OAuth設定管理モジュール** (`src-tauri/src/gmail/config.rs`)
   - keyringを使用した認証情報の保存・読込・削除
   - JSON形式からの認証情報抽出（"installed"と"web"両方に対応）
   - 単体テスト実装

2. **gmailモジュールの再構成**
   - `gmail.rs` → `gmail/client.rs`にリネーム
   - `gmail/mod.rs`で公開APIを管理
   - 全ての公開関数・型をre-export

3. **GmailClientの認証フロー修正** (`src-tauri/src/gmail/client.rs`)
   - keyringから`client_id`と`client_secret`を読み込み
   - `ApplicationSecret`を手動で構築
   - ファイルベースの認証情報読み込みを廃止

4. **Tauriコマンド追加** (`src-tauri/src/lib.rs`)
   - `has_gmail_oauth_credentials`
   - `save_gmail_oauth_credentials`
   - `delete_gmail_oauth_credentials`

5. **フロントエンド設定画面** (`src/components/screens/settings.tsx`)
   - Gmail OAuth認証セクション追加
   - JSON貼り付け/ファイルアップロードの切り替え
   - 保存・削除機能

6. **UIコンポーネント追加**
   - `src/components/ui/textarea.tsx`（新規作成）

### 検証結果

- Rust lint (clippy): OK
- Rust format: OK
- Rust tests: 304 passed
- Frontend lint: OK
- Frontend tests: 376 passed
