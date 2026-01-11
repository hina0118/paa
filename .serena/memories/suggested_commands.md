# 推奨コマンド

## 開発コマンド

### 開発サーバー起動
```bash
npm run dev
```
Vite開発サーバーとTauriアプリを起動します。
- Viteは http://localhost:1420 で起動
- ホットリロード (HMR) が有効

### Tauri開発モード
```bash
npm run tauri dev
```
Tauriの開発モードを直接起動します。

### ビルド
```bash
npm run build
```
1. TypeScriptのコンパイル (`tsc`)
2. Viteによる本番ビルド

本番用のバンドルを生成します。

### プレビュー
```bash
npm run preview
```
ビルドされたアプリをプレビューします。

## パッケージ管理

### 依存関係のインストール
```bash
npm install
```

### shadcn/uiコンポーネントの追加
```bash
npx shadcn@latest add <component-name>
```
例: `npx shadcn@latest add button`

## Windows固有のユーティリティコマンド

### ディレクトリ一覧
```cmd
dir
```

### ディレクトリ移動
```cmd
cd <path>
```

### ファイル検索
```cmd
dir /s /b <filename>
```

### 文字列検索
```cmd
findstr /s /i "pattern" *.ts *.tsx
```

### プロセス確認
```cmd
tasklist
```

### ポート使用状況確認
```cmd
netstat -ano | findstr :1420
```

## Git コマンド

### ステータス確認
```bash
git status
```

### 変更の追加
```bash
git add .
```

### コミット
```bash
git commit -m "message"
```

### プッシュ
```bash
git push
```

## トラブルシューティング

### node_modules再インストール
```bash
rmdir /s /q node_modules
npm install
```

### Tauriキャッシュクリア
```bash
cd src-tauri
cargo clean
cd ..
```

### 開発サーバーのポートが使用中の場合
```bash
# プロセスを確認
netstat -ano | findstr :1420
# タスクキル (PIDを確認後)
taskkill /PID <pid> /F
```
