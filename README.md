# Purchase Archive & Assistant (PAA)

10年以上の買い物履歴を「資産」に変え、現在の買い物をサポートするパーソナル・アシスタント。

## システム概要

- **プラットフォーム**: Windows (Tauri 2 + Rust + React + TypeScript)
- **常駐形態**: タスクトレイ常駐型デスクトップアプリ
- **データベース**: SQLite (ローカル完結、オフライン動作)

## 主要機能

### データ収集 (Data Ingestion)

- **Gmail 同期**: Gmail API 経由で対応 EC サイトのメールをフィルタリングして取得・保存
- **駿河屋 WebView セッション**: ログイン済み WebView ウィンドウ経由でマイページ HTML を取得し、注文データを補完

### メール解析 (Parsing)

- **プラグイン型パーサー**: 店舗ごとのメール種別を `VendorPlugin` トレイトで抽象化し、`inventory::submit!` で自動登録
- **対応店舗**:
  - アミアミ (通常 / 楽天市場経由)
  - アニメイト
  - DMM (注文確認 / 発送 / キャンセル / 注文番号変更 / まとめ / 分割完了)
  - フルイチオンライン
  - グッドスマイルカンパニー
  - ホビーサーチ (予約 / 変更 / 発送 / キャンセル)
  - キッズドラゴン
  - プレミアムバンダイ (まとめ注文対応)
  - 佐川急便 (配達完了メール)
  - 駿河屋 / 駿河屋マーケットプレイス
- **特殊処理**: キャンセル・注文番号変更・まとめ完了など、`OrderInfo` を返さない種別も専用 `DispatchOutcome` で処理

### 商品管理 (Product Management)

- **商品マスター編集**: 商品名・正規化名・購入除外フラグなどを画面から編集
- **商品名 AI 解析**: Google Gemini API を使って商品名を正規化・分類
- **手動オーバーライド**: 商品名・価格・配送情報を手動で上書き可能
- **除外設定**: 特定商品・注文をリストから除外

### 画像管理 (Image Management)

- **SerpApi 画像検索**: 商品名をもとに Google 画像検索 API から画像を取得・DB 保存
- **Google 画像検索**: 商品名でブラウザの Google 画像検索を開く

### 配送管理 (Delivery Tracker)

- **配送状況追跡**: 追跡番号をもとに配送状況を確認・記録
- **配達完了検出**: 佐川急便の配達完了メールから自動でステータスを更新
- **追跡ログ**: `tracking_check_logs` テーブルに確認履歴を保存

### OCR・購入確認アシスタント

- **画面 OCR**: Windows OCR (`Windows.Media.Ocr`) を利用し、半透明オーバーレイ上で画面上の商品名を読み取り
- **購入履歴照合**: OCR 結果で商品一覧を自動検索（`ocr-result` イベント経由）
- **通知**: 照合結果を Windows のトースト通知で表示

### 自動化・スケジューラ

- **バックグラウンドスケジューラ**: 差分同期 → メールパース → 商品名解析 → 配達状況確認のパイプラインを一定間隔で自動実行
- **トレイメニュー**: スケジューラの有効/無効切り替え、同期・OCR スキャンへのクイックアクセス
- **多重実行防止**: パイプライン実行中は次の tick をスキップ

## 画面構成

| 画面 | 内容 |
| --- | --- |
| ダッシュボード | 注文数・商品数・配送統計などのサマリー |
| 商品一覧 (orders) | 商品カード一覧・OCR 検索結果表示 |
| 配送管理 (deliveries) | 配送状況の一覧・追跡 |
| バッチ実行 (batch) | Gmail 同期・メールパース・商品名解析を手動実行 |
| ショップ設定 | 送信元アドレス・件名フィルター・パーサー種別の管理 |
| 商品マスター | 商品正規化名・除外フラグの編集 |
| API キー | Gemini API / SerpApi キーの設定 |
| 設定 | スケジューラ間隔・外観・バックアップなど |
| ログ | アプリ内ログの表示 |
| テーブルビュー | 各 DB テーブルの内容を直接確認 |

## データベース主要テーブル

| テーブル | 内容 |
| --- | --- |
| `emails` | Gmail から取得したメール |
| `orders` | 注文単位の情報 |
| `items` | 注文内の個別商品 |
| `deliveries` | 配送情報・追跡番号 |
| `images` | 商品画像ファイル名 |
| `htmls` | 取得した HTML |
| `shop_settings` | ショップごとのパーサー設定 |
| `product_master` | 商品マスター（正規化名・除外フラグ） |
| `item_overrides` | 商品情報の手動オーバーライド |
| `order_overrides` | 注文情報の手動オーバーライド |
| `excluded_items` | 除外商品リスト |
| `excluded_orders` | 除外注文リスト |
| `tracking_check_logs` | 配送追跡確認ログ |

## タイムゾーン規約

アプリ全体で **日本標準時 (JST)** を表示用に統一する。

| レイヤー | 実装 | 用途 |
| --- | --- | --- |
| バックエンド (Rust) | `chrono_tz::Asia::Tokyo` | ログ出力のタイムスタンプ |
| フロントエンド (TypeScript) | `'Asia/Tokyo'` (Intl) | 日付・日時の表示フォーマット |
| データベース | UTC | 保存形式（SQLite の日時カラム） |

---

## Gmail API セットアップ

### 1. Google Cloud Console でプロジェクトを作成

1. [Google Cloud Console](https://console.cloud.google.com/) にアクセス
2. 新しいプロジェクトを作成
3. 「API とサービス」→「ライブラリ」から「Gmail API」を有効化

### 2. OAuth 2.0 クライアント ID の作成

1. 「API とサービス」→「認証情報」
2. 「認証情報を作成」→「OAuth クライアント ID」
3. アプリケーションの種類: 「デスクトップアプリ」
4. 名前: 任意（例: PAA Desktop Client）
5. 作成後、JSON をダウンロード

### 3. クライアントシークレットファイルの配置

ダウンロードした JSON ファイルを **DBファイルと同じディレクトリ** に `client_secret.json` として配置:

```
%APPDATA%\jp.github.hina0118.paa\client_secret.json
```

> ヒント: エクスプローラーのアドレスバーに `%APPDATA%\jp.github.hina0118.paa` と入力するとディレクトリに直接アクセスできます。

### 4. 初回認証

アプリケーションで「Gmail 同期」を実行すると認証が必要です。

1. 「Gmail 同期」ボタンをクリック
2. ブラウザが自動的に開いて Google の認証画面が表示される
3. アカウントを選択してアクセスを許可
4. 認証完了後、トークンが自動保存され次回以降は不要

> ブラウザが開かない場合: F12 → コンソールに表示される認証 URL を手動でブラウザに貼り付け

---

## 新しい店舗（EC サイト）を追加する

プラグイン設計により、**変更箇所は最小限**で新店舗に対応できます。

### 手順

**1. プラグインを実装し、`inventory::submit!` で自動登録する**

`src-tauri/src/plugins/<店舗名>/mod.rs` を作成し、`VendorPlugin` トレイトを実装します。

```rust
pub struct NewShopPlugin;

impl VendorPlugin for NewShopPlugin {
    fn parser_types(&self) -> &[&str] {
        &["newshop_confirm", "newshop_send", "newshop_cancel"]
    }

    fn priority(&self) -> i32 { 10 }

    fn shop_name(&self) -> &str { "新店舗名" }

    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> { ... }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        // 送信元アドレス・件名フィルター・parser_type のデフォルト設定を返す
        // アプリ起動時に DB へ自動挿入される（INSERT OR IGNORE）
        vec![ ... ]
    }

    async fn dispatch(&self, parser_type: &str, ...) -> Result<DispatchOutcome, DispatchError> { ... }
}

// ファイル末尾に追加するだけで自動登録される
inventory::submit! {
    crate::plugins::PluginRegistration {
        factory: || Box::new(NewShopPlugin),
    }
}
```

**2. `plugins/mod.rs` に `pub mod` を追加する**

```rust
// src-tauri/src/plugins/mod.rs
pub mod newshop;  // pub mod にすることで LTO でも自動登録が除外されない
```

**3. 動作確認する**

```bash
cargo test
```

アプリを起動すると `ensure_default_settings()` が自動実行され、`default_shop_settings()` で定義したレコードが `shop_settings` テーブルへ挿入されます。

---

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

#### 全テスト実行

```bash
npm run test:all
```

### Lint

| コマンド | 内容 |
| --- | --- |
| `npm run lint` | Rust・UI・フォーマットをまとめて実行（CI 想定） |
| `npm run lint:rust` | Rust 用（Clippy、全ターゲット・全機能、警告をエラー扱い） |
| `npm run lint:rust:fix` | Rust 用の自動修正 |
| `npm run lint:ui` | フロント用 ESLint（--max-warnings 4） |
| `npm run lint:ui:fix` | フロント用 ESLint の自動修正 |
| `npm run format:check` | Prettier のチェックのみ（書き換えしない） |
| `npm run format` | Prettier でフォーマット（書き換えする） |
| `npm run lint:fix` | lint:rust:fix + lint:ui:fix + format をまとめて実行 |

---

## PR レビューコメントの解決

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

| パラメータ | 説明 |
| --- | --- |
| `-PrNumber` | PR 番号（必須。`-CurrentBranch` 使用時は不要） |
| `-Owner` | リポジトリオーナー（省略時は `gh repo view` または git remote から取得） |
| `-Repo` | リポジトリ名（省略時は同上） |
| `-CurrentBranch` | 現在のブランチの PR を対象にする |
| `-ThreadIds` | 解決するスレッド ID の配列（省略時は未解決スレッドを自動取得） |
