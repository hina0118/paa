# コーディングスタイルと規約

## TypeScript設定

### Strictモード
- `strict: true` - 厳格な型チェックが有効
- `noUnusedLocals: true` - 未使用のローカル変数はエラー
- `noUnusedParameters: true` - 未使用のパラメータはエラー
- `noFallthroughCasesInSwitch: true` - switch文のフォールスルーはエラー

### モジュール
- ESNext モジュールシステムを使用
- `import`/`export` 構文を使用

## ファイル構成規約

### インポート順序
1. 外部ライブラリ (React, Tauriなど)
2. `@/` エイリアスを使用した内部モジュール
3. 相対パスのインポート

### パスエイリアス
- `@/` → `./src/` 
  - 例: `@/components/ui/button`
  - 例: `@/lib/utils`

## React規約

### コンポーネント定義
- 関数コンポーネントを使用
- 名前付きエクスポートまたはデフォルトエクスポート
- PascalCaseでコンポーネント名を定義

例:
```typescript
function MyComponent() {
  return <div>...</div>;
}

export default MyComponent;
```

### Hooks
- `use` プレフィックスを使用
- カスタムフックは `contexts/` または `hooks/` に配置

### 型定義
- インターフェースや型は `lib/types.ts` に集約
- コンポーネント固有の型はファイル内で定義可能

## スタイリング規約

### Tailwind CSS
- ユーティリティクラスを使用
- `className` プロパティで指定
- 複雑な条件付きクラスには `clsx` や `cn` ヘルパーを使用

### CSS変数
- テーマカラーはCSS変数 (`hsl(var(--primary))`) を使用
- `index.css` でグローバル変数を定義

### shadcn/uiスタイル
- "New York" スタイルを採用
- コンポーネントは `src/components/ui/` に配置
- `class-variance-authority` を使用したバリアント管理

## 命名規約

### ファイル名
- コンポーネント: `kebab-case.tsx` (例: `email-list.tsx`)
- ユーティリティ: `kebab-case.ts` (例: `utils.ts`)
- 型定義: `types.ts`

### 変数・関数
- camelCase を使用
- 定数は UPPER_SNAKE_CASE も可

### 型・インターフェース
- PascalCase を使用
- 例: `Email`, `NavigationContextType`

## その他の規約

### JSX
- `react-jsx` トランスフォームを使用 (React 17+)
- `React` のインポートは不要

### エラーハンドリング
- TypeScriptの型チェックに依存
- 必要に応じて try-catch を使用

### コメント
- 複雑なロジックには適宜コメントを追加
- TSDocスタイルのコメントは推奨
