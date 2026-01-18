# コードベース構造

## ディレクトリ構成
```
paa/
├── src/                          # フロントエンドソース
│   ├── components/               # Reactコンポーネント
│   │   ├── emails/               # メール関連コンポーネント
│   │   │   ├── columns.tsx       # テーブルカラム定義
│   │   │   ├── data-table.tsx    # データテーブル
│   │   │   └── email-list.tsx    # メールリスト
│   │   ├── layout/               # レイアウトコンポーネント
│   │   │   └── sidebar.tsx       # サイドバー
│   │   ├── screens/              # 画面コンポーネント
│   │   │   ├── dashboard.tsx     # ダッシュボード
│   │   │   ├── settings.tsx      # 設定画面
│   │   │   └── sync.tsx          # 同期画面
│   │   └── ui/                   # UIコンポーネント (shadcn/ui)
│   │       ├── button.tsx
│   │       ├── checkbox.tsx
│   │       ├── dropdown-menu.tsx
│   │       ├── input.tsx
│   │       └── table.tsx
│   ├── contexts/                 # Reactコンテキスト
│   │   └── navigation-context.tsx # ナビゲーション状態管理
│   ├── lib/                      # ユーティリティ・型定義
│   │   ├── data.ts               # データ定義
│   │   ├── types.ts              # 型定義 (Email型など)
│   │   └── utils.ts              # ユーティリティ関数
│   ├── test/                     # テスト設定
│   │   └── setup.ts              # テスト環境設定・モック
│   ├── assets/                   # 静的アセット
│   ├── App.tsx                   # メインアプリケーション
│   ├── main.tsx                  # エントリーポイント
│   ├── index.css                 # グローバルスタイル
│   └── vite-env.d.ts             # Vite型定義
├── src-tauri/                    # Tauriバックエンド (Rust)
│   ├── src/                      # Rustソースコード
│   │   ├── gmail.rs              # Gmail API統合 (テストモジュール含む)
│   │   ├── lib.rs                # Tauriコマンド定義
│   │   └── main.rs               # エントリーポイント
│   ├── tests/                    # 統合テスト
│   │   └── command_tests.rs      # コマンドテスト
│   ├── Cargo.toml                # Rustプロジェクト設定
│   ├── TESTING.md                # テストガイド
│   └── gen/schemas/              # 生成されたスキーマ
├── coverage/                     # カバレッジレポート (gitignore)
│   └── index.html                # フロントエンドカバレッジHTML
├── public/                       # 公開静的ファイル
├── .vscode/                      # VS Code設定
├── components.json               # shadcn/ui設定
├── tailwind.config.js            # Tailwind CSS設定
├── tsconfig.json                 # TypeScript設定
├── vite.config.ts                # Vite設定
└── package.json                  # npm設定
```

## 主要なアーキテクチャパターン

### ナビゲーション
- `NavigationContext` を使用したコンテキストベースのナビゲーション
- 画面種類: "dashboard", "orders", "sync", "settings"
- サイドバーからの画面切り替え

### コンポーネント構成
- `App.tsx`: NavigationProviderでラップされたルートコンポーネント
- `AppContent`: ナビゲーション状態に応じて画面をレンダリング
- レイアウト: Flexboxベースの横並び (Sidebar + Main)

### データ型
- `Email`: id, from, subject, preview, date, read, starred, labels

### スタイリング
- Tailwind CSSのユーティリティクラス
- CSS変数ベースのテーマシステム
- shadcn/uiの"New York"スタイル
