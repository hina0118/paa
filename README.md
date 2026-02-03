# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

# Purchase Archive & Assistant (PAA) 設計仕様書

10年以上の買い物履歴を「資産」に変え、現在の買い物をサポートするパーソナル・アシスタント。

## 1. システム概要

- **プラットフォーム**: Windows (Tauri + Rust)
- **常駐形態**: タスクトレイ常駐型
- **データベース**: SQLite (ローカル完結、オフライン動作)

## 2. 主要機能

### 2.1 データ収集 (Data Ingestion)

- **Gmail同期**: API経由で特定ECサイトのメールをフィルタリング取得。
  - 保存データ: メール本文 (Raw Body), メールのメタデータ。
- **注文詳細保存**: アプリ内WebView(Tauri)経由でECサイトの注文詳細ページを取得。
  - 保存データ: HTML (Raw HTML)。
- **外部画像連携**: 商品名をもとに画像検索APIから画像を取得。
  - 保存データ: 画像バイナリ (BLOB) をDBに直接保存。

### 2.2 解析・管理 (Parsing & Management)

- **情報の統合 (Merge)**: メール(受信日等)とHTML(正式名・価格・追跡番号)をマージ。
- **解析ロジックの分離**: 各サイトのパーサーを独立させ、正規表現やセレクタをアプリ画面から編集可能にする。
- **商品ベース管理**: 1注文内にある複数商品を個別のレコードとして管理。

### 2.3 配送管理 (Delivery Tracker)

- **追跡番号抽出**: 保存済みHTMLから自動で運送会社と追跡番号を特定。
- **自動更新**: 配送会社の追跡ページを定期的にバックグラウンドで確認し、ステータスを更新。

### 2.4 OCR・購入確認アシスタント

- **Windows OCR**: `Windows.Media.Ocr` を利用し、画面上の商品名を読み取り。
- **購入チェック**: `Alt + S` などのホットキーで画面をスキャンし、DBと照合。
  - **曖昧検索**: 文字揺れやノイズ(送料込等)を許容するFuzzy Search。
- **通知**: 照合結果をWindowsのトースト通知で即座に表示。

## 3. データベース構造 (主要テーブル)

### `orders` (注文単位)

- `id`, `gmail_message_id`, `shop_domain`, `raw_body`, `raw_html`, `order_date`

### `items` (商品単位)

- `id`, `order_id` (FK), `item_name`, `price`, `tracking_number`, `delivery_status`

### `images` (画像データ)

- `item_id` (FK), `file_name` (TEXT)

## 4. タイムゾーン規約

アプリ全体で **日本標準時 (JST)** を表示用に統一する。

| レイヤー                    | 実装                     | 用途                            |
| --------------------------- | ------------------------ | ------------------------------- |
| バックエンド (Rust)         | `chrono_tz::Asia::Tokyo` | ログ出力のタイムスタンプ        |
| フロントエンド (TypeScript) | `'Asia/Tokyo'` (Intl)    | 日付・日時の表示フォーマット    |
| データベース                | UTC                      | 保存形式（SQLite の日時カラム） |

他タイムゾーンを追加する場合は、Rust では `chrono_tz`、フロントでは IANA 文字列（例: `'America/New_York'`）で同一のタイムゾーンを指定すること。

## 5. UI/UX 仕様

- **メイン画面**: 商品画像を中心としたカード型グリッド表示。10年分の履歴を高速スクロール可能。
- **トレイメニュー**: 同期実行、画面スキャン、設定へのクイックアクセス。
- **進捗表示**: 「メール同期済み」「解析済み」「配送中」などのフェーズを視覚化。

## Gmail API セットアップ

このアプリケーションはGmail APIを使用して注文メールを取得します。以下の手順でセットアップしてください。

### 1. Google Cloud Consoleでプロジェクトを作成

1. [Google Cloud Console](https://console.cloud.google.com/)にアクセス
2. 新しいプロジェクトを作成
3. 「APIとサービス」→「ライブラリ」から「Gmail API」を有効化

### 2. OAuth 2.0 クライアント IDの作成

1. 「APIとサービス」→「認証情報」
2. 「認証情報を作成」→「OAuth クライアント ID」
3. アプリケーションの種類: 「デスクトップアプリ」
4. 名前: 任意（例: PAA Desktop Client）
5. 作成後、JSONをダウンロード

### 3. クライアントシークレットファイルの配置

ダウンロードしたJSONファイルを **DBファイルと同じディレクトリ** に `client_secret.json` として配置:

**配置場所**:

```
%APPDATA%\jp.github.hina0118.paa\client_secret.json
```

**パスの例**:

```
C:\Users\<ユーザー名>\AppData\Roaming\jp.github.hina0118.paa\client_secret.json
```

**注意**: このディレクトリには `paa_data.db` (データベースファイル) も保存されます。すべてのアプリケーションデータが同じ場所に集約されます。

**ヒント**: エクスプローラーのアドレスバーに `%APPDATA%\jp.github.hina0118.paa` と入力するとディレクトリに直接アクセスできます。

### 4. 初回認証

アプリケーションで初めて「メール取得」機能を実行すると、認証が必要です。

**認証手順**:

1. 「Gmailメールを取得」ボタンをクリック
2. 開発モードの場合、以下のいずれかの方法で認証:

   **方法A: ブラウザが自動で開く場合**
   - ブラウザが自動的に開いてGoogleの認証画面が表示されます
   - アカウントを選択して、アプリケーションへのアクセスを許可

   **方法B: ブラウザが開かない場合**
   - 開発者ツール（F12キー）を開く
   - コンソールタブに表示される認証URLをコピー
   - ブラウザで手動で開く
   - アカウントを選択して、アプリケーションへのアクセスを許可
   - ブラウザが `http://localhost:8080/?code=...` のようなURLにリダイレクトされる（これは正常な動作）

3. 認証を完了すると、トークンが自動的に保存され、次回以降は認証不要になります。

## 開発用コマンド

### アプリケーションの起動

```bash
npm run tauri dev
```

### テスト

**目標カバレッジ: 85%**

#### バックエンド（Rust）テスト

```bash
npm run test
# または
cd src-tauri && cargo test
```

カバレッジ計測:

```bash
npm run test:coverage
# または
cd src-tauri && cargo llvm-cov --all-features --workspace --html
```

HTMLレポート: `src-tauri/target/llvm-cov/html/index.html`

詳細: `src-tauri/TESTING.md`

#### フロントエンド（React）テスト

```bash
npm run test:frontend
# または
npm run test:frontend:run  # 一度だけ実行
```

カバレッジ計測:

```bash
npm run test:frontend:coverage
```

HTMLレポート: `coverage/index.html`

詳細: `TESTING_FRONTEND.md`

#### 全てのテスト実行

```bash
npm run test:all
```

#### Lint

npm スクリプト（プロジェクトルートで実行）

| コマンド                | 内容                                                      |
| ----------------------- | --------------------------------------------------------- |
| `npm run lint`          | Rust・UI・フォーマットをまとめて実行（CI 想定）           |
| `npm run lint:rust`     | Rust 用（Clippy、全ターゲット・全機能、警告をエラー扱い） |
| `npm run lint:rust:fix` | Rust 用の自動修正                                         |
| `npm run lint:ui`       | フロント用 ESLint（--max-warnings 4）                     |
| `npm run lint:ui:fix`   | フロント用 ESLint の自動修正                              |
| `npm run format:check`  | Prettier のチェックのみ（書き換えしない）                 |
| `npm run format`        | Prettier でフォーマット（書き換えする）                   |
| `npm run lint:fix`      | lint:rust:fix ＋ lint:ui:fix ＋ format をまとめて実行     |

### PR レビューコメントの解決

対応済みのレビューコメントを GitHub 上で Resolved にするスクリプトです。

**前提**: [GitHub CLI (gh)](https://cli.github.com/) がインストール済みで `gh auth login` 済みであること。

```powershell
# 最もシンプル（リポジトリ内で実行、未解決スレッドを自動取得して解決）
.\scripts\resolve-pr-review-threads.ps1 -PrNumber 59

# リポジトリ外から実行する場合
.\scripts\resolve-pr-review-threads.ps1 -Owner hina0118 -Repo paa -PrNumber 59

# 現在のブランチの PR を一括解決
.\scripts\resolve-pr-review-threads.ps1 -CurrentBranch

# スレッド ID を明示指定する場合
.\scripts\resolve-pr-review-threads.ps1 -PrNumber 59 -ThreadIds @("PRRT_xxx","PRRT_yyy")
```

| パラメータ       | 説明                                                                     |
| ---------------- | ------------------------------------------------------------------------ |
| `-PrNumber`      | PR 番号（必須。`-CurrentBranch` 使用時は不要）                           |
| `-Owner`         | リポジトリオーナー（省略時は `gh repo view` または git remote から取得） |
| `-Repo`          | リポジトリ名（省略時は同上）                                             |
| `-CurrentBranch` | 現在のブランチの PR を対象にする                                         |
| `-ThreadIds`     | 解決するスレッド ID の配列（省略時は未解決スレッドを自動取得）           |
