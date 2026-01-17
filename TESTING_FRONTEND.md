# フロントエンドテストガイド

このドキュメントでは、フロントエンド（React + TypeScript）のテスト実行とカバレッジ計測の方法を説明します。

## テストスタック

- **テストフレームワーク**: [Vitest](https://vitest.dev/)
- **テスティングライブラリ**: [React Testing Library](https://testing-library.com/react)
- **DOM環境**: jsdom
- **カバレッジ**: @vitest/coverage-v8

## テスト実行

### 基本的なテスト実行

ウォッチモード（開発時）:
```bash
npm run test:frontend
```

一度だけ実行（CI/CD用）:
```bash
npm run test:frontend:run
```

UIモード（ビジュアルテストランナー）:
```bash
npm run test:frontend:ui
```

### カバレッジ計測

カバレッジレポート生成:
```bash
npm run test:frontend:coverage
```

HTMLレポートは `coverage/index.html` に生成されます。

### 全てのテスト実行（フロント + バックエンド）

```bash
npm run test:all
```

## テストの書き方

### コンポーネントテスト

React Componentのテスト例（`button.test.tsx`）:

```tsx
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Button } from './button'

describe('Button', () => {
  it('renders button with text', () => {
    render(<Button>Click me</Button>)
    expect(screen.getByRole('button', { name: /click me/i })).toBeInTheDocument()
  })

  it('handles click events', async () => {
    const handleClick = vi.fn()
    const user = userEvent.setup()

    render(<Button onClick={handleClick}>Click me</Button>)

    await user.click(screen.getByRole('button'))
    expect(handleClick).toHaveBeenCalledTimes(1)
  })
})
```

### ユーティリティ関数のテスト

純粋関数のテスト例（`utils.test.ts`）:

```ts
import { describe, it, expect } from 'vitest'
import { cn } from './utils'

describe('cn utility', () => {
  it('merges class names correctly', () => {
    const result = cn('text-red-500', 'bg-blue-500')
    expect(result).toBe('text-red-500 bg-blue-500')
  })
})
```

### Tauri APIのモック

Tauri APIは自動的にモック化されます（`src/test/setup.ts`で設定）:

```tsx
import { mockInvoke } from '@/test/setup'

describe('Component with Tauri API', () => {
  it('calls Tauri command', async () => {
    mockInvoke.mockResolvedValueOnce({ success: true })

    // テストコード

    expect(mockInvoke).toHaveBeenCalledWith('command_name', { param: 'value' })
  })
})
```

## テストのベストプラクティス

### 1. AAA パターン（Arrange-Act-Assert）

```tsx
it('updates count when button is clicked', async () => {
  // Arrange: テストの準備
  const user = userEvent.setup()
  render(<Counter />)

  // Act: アクション実行
  await user.click(screen.getByRole('button', { name: /increment/i }))

  // Assert: 結果の検証
  expect(screen.getByText('Count: 1')).toBeInTheDocument()
})
```

### 2. ユーザー中心のクエリを使用

優先順位:
1. `getByRole` - アクセシビリティを考慮
2. `getByLabelText` - フォーム要素
3. `getByPlaceholderText` - 入力フィールド
4. `getByText` - 表示テキスト
5. `getByTestId` - 最後の手段

```tsx
// ✅ Good
screen.getByRole('button', { name: /submit/i })

// ❌ Bad
screen.getByTestId('submit-button')
```

### 3. 非同期操作のハンドリング

```tsx
it('loads data asynchronously', async () => {
  render(<DataComponent />)

  // 要素が表示されるまで待機
  const heading = await screen.findByRole('heading', { name: /data loaded/i })
  expect(heading).toBeInTheDocument()
})
```

### 4. モックの適切な使用

```tsx
import { vi } from 'vitest'

it('calls API on submit', async () => {
  const mockFetch = vi.fn().mockResolvedValue({ ok: true })
  global.fetch = mockFetch

  // テストコード

  expect(mockFetch).toHaveBeenCalledWith('/api/endpoint', expect.any(Object))
})
```

## カバレッジ目標

- **全体目標**: 85%
- **重要コンポーネント**: 90%以上
- **ユーティリティ関数**: 100%

### 除外対象
- `src/test/` - テストファイル自体
- `**/*.config.{js,ts}` - 設定ファイル
- `**/dist/**` - ビルド成果物
- `**/*.d.ts` - 型定義ファイル

## ディレクトリ構造

```
src/
├── components/
│   ├── ui/
│   │   ├── button.tsx
│   │   └── button.test.tsx         # コンポーネントテスト
│   ├── screens/
│   └── layout/
├── lib/
│   ├── utils.ts
│   └── utils.test.ts                # ユーティリティテスト
├── test/
│   └── setup.ts                     # テスト設定
└── contexts/
```

## CI/CD統合

GitHub Actionsの例:

```yaml
- name: Install dependencies
  run: npm ci

- name: Run frontend tests
  run: npm run test:frontend:run

- name: Run frontend tests with coverage
  run: npm run test:frontend:coverage

- name: Upload coverage to Codecov
  uses: codecov/codecov-action@v3
  with:
    files: ./coverage/lcov.info
    flags: frontend
```

## トラブルシューティング

### jsdomエラーが出る場合

`vitest.config.ts`で環境が正しく設定されているか確認:

```ts
export default defineConfig({
  test: {
    environment: 'jsdom',
  },
})
```

### Tailwind CSSのクラスが正しくマージされない場合

`cn`関数のテストで`tailwind-merge`が正しく動作しているか確認。

### モックが動作しない場合

`src/test/setup.ts`でモックが正しく設定されているか確認:

```ts
vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}))
```

## リソース

- [Vitest Documentation](https://vitest.dev/)
- [React Testing Library](https://testing-library.com/react)
- [Testing Library Queries](https://testing-library.com/docs/queries/about)
- [Common Testing Mistakes](https://kentcdodds.com/blog/common-mistakes-with-react-testing-library)

## 現在のテスト状況

- **テストファイル数**: 2
- **テスト数**: 16
- **カバレッジ**: 100% ✅

### テスト済みファイル
- ✅ `src/components/ui/button.tsx` - 9テスト
- ✅ `src/lib/utils.ts` - 7テスト

### 今後追加すべきテスト
1. **画面コンポーネント**
   - Dashboard
   - Settings
   - Sync

2. **データテーブル**
   - EmailList
   - DataTable
   - Columns

3. **Context**
   - NavigationContext
   - SyncContext

4. **統合テスト**
   - フロー全体のテスト
   - Tauri APIとの連携
