# 技術スタック

## フロントエンド
- **フレームワーク**: React 19.1.0
- **言語**: TypeScript 5.8.3
- **ビルドツール**: Vite 7.0.4
- **スタイリング**: 
  - Tailwind CSS 3.4.19
  - PostCSS 8.5.6
  - Autoprefixer 10.4.23
- **UIライブラリ**: 
  - shadcn/ui (New York スタイル)
  - Radix UI (@radix-ui/react-checkbox, @radix-ui/react-dropdown-menu)
  - Lucide React (アイコン)
- **データテーブル**: @tanstack/react-table 8.21.3
- **ユーティリティ**: 
  - clsx 2.1.1
  - tailwind-merge 3.4.0
  - class-variance-authority 0.7.1

## バックエンド
- **フレームワーク**: Tauri 2
- **言語**: Rust (edition 2021)
- **プラグイン**: 
  - tauri-plugin-opener
- **シリアライゼーション**: 
  - serde 1.x
  - serde_json 1.x

## 開発環境
- **パッケージマネージャ**: npm
- **推奨IDE**: VS Code
  - 推奨拡張機能: Tauri, rust-analyzer
- **OS**: Windows

## TypeScript設定
- **ターゲット**: ES2020
- **モジュール**: ESNext
- **JSX**: react-jsx
- **Strict モード**: 有効
- **パスエイリアス**: `@/*` → `./src/*`

## Vite設定
- **開発サーバーポート**: 1420 (固定)
- **HMRポート**: 1421
- **src-tauriディレクトリは監視対象外**
