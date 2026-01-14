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
- `item_id` (FK), `image_data` (BLOB), `source_url`

## 4. UI/UX 仕様
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

ダウンロードしたJSONファイルを以下の場所に `client_secret.json` として配置:

```
%APPDATA%\com.tauri.dev\client_secret.json
```

パスの例:
```
C:\Users\<ユーザー名>\AppData\Roaming\com.tauri.dev\client_secret.json
```

### 4. 初回認証

アプリケーションで初めて「メール取得」機能を実行すると、ブラウザが開いてGoogleアカウントの認証画面が表示されます。認証を完了すると、トークンが自動的に保存され、次回以降は認証不要になります。

## 開発用コマンド

npm run tauri dev
