# テストベストプラクティス

## フロントエンド (React + TypeScript)

### テストの書き方

#### 1. AAA パターン (Arrange-Act-Assert)
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

#### 2. ユーザー中心のクエリを使用
**優先順位:**
1. `getByRole` - アクセシビリティを考慮 (推奨)
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

#### 3. 非同期操作のハンドリング
```tsx
it('loads data asynchronously', async () => {
  render(<DataComponent />)

  // 要素が表示されるまで待機
  const heading = await screen.findByRole('heading', { name: /data loaded/i })
  expect(heading).toBeInTheDocument()
})
```

#### 4. Tauri APIのモック
`src/test/setup.ts`で自動的にモック化されます:

```tsx
import { mockInvoke } from '@/test/setup'

it('calls Tauri command', async () => {
  mockInvoke.mockResolvedValueOnce({ success: true })

  // テストコード

  expect(mockInvoke).toHaveBeenCalledWith('command_name', { param: 'value' })
})
```

#### 5. ファイル命名規則
- テストファイルは対象ファイルと同じディレクトリに配置
- ファイル名: `<filename>.test.tsx` または `<filename>.test.ts`

### カバレッジ目標
- **全体目標**: 85%
- **重要コンポーネント**: 90%以上
- **ユーティリティ関数**: 100%

### 除外対象
- `src/test/` - テストファイル自体
- `**/*.config.{js,ts}` - 設定ファイル
- `**/dist/**` - ビルド成果物
- `**/*.d.ts` - 型定義ファイル

## バックエンド (Rust)

### テストの書き方

#### 1. 単体テスト (ファイル内モジュール)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        let result = my_function(input);
        assert_eq!(result, expected);
    }
}
```

#### 2. 非同期テスト
```rust
#[tokio::test]
async fn test_async_function() {
    let pool = create_test_db().await;
    let result = async_operation(&pool).await.unwrap();
    assert_eq!(result, expected);
}
```

#### 3. インメモリデータベーステスト
```rust
async fn create_test_db() -> sqlx::SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .expect("Failed to create test database");

    // テーブル作成
    sqlx::query("CREATE TABLE IF NOT EXISTS ...")
        .execute(&pool)
        .await
        .expect("Failed to create table");

    pool
}
```

#### 4. エラーケースのテスト
```rust
#[tokio::test]
async fn test_error_handling() {
    let pool = create_test_db().await;
    
    let result = function_that_might_fail(&pool, invalid_input).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Expected error message"));
}
```

#### 5. 境界値テスト
```rust
#[test]
fn test_boundary_values() {
    assert_eq!(validate_batch_size(0), false);
    assert_eq!(validate_batch_size(1), true);
    assert_eq!(validate_batch_size(1000), true);
    assert_eq!(validate_batch_size(1001), false);
}
```

### テストの配置

#### ファイル内テスト
```rust
// src/gmail.rs の最後
#[cfg(test)]
mod tests {
    // 単体テスト
}
```

#### 統合テスト
```
src-tauri/
└── tests/
    └── command_tests.rs  // 複数モジュールを統合したテスト
```

### カバレッジ目標
- **全体目標**: 85%
- **ビジネスロジック**: 90%以上
- **データベース操作**: 85%以上
- **エントリーポイント**: 測定対象外 (main.rs)

### テストが難しい部分の対処
1. **OAuth認証**: モックフレームワーク (`mockito`, `wiremock`) を使用
2. **Gmail API呼び出し**: モックレスポンスを返すテストヘルパー
3. **Tauriランタイム**: `tauri::test` モジュールを活用

## 共通のベストプラクティス

### 1. テストは独立している
- 各テストは他のテストに依存しない
- テストの実行順序に依存しない
- テストごとに環境をセットアップ・クリーンアップ

### 2. テストは読みやすい
- テスト名は動作を明確に説明
- Given-When-Then パターンを使用
- マジックナンバーを避ける

### 3. テストは保守しやすい
- DRY原則: ヘルパー関数を活用
- テストデータはビルダーパターンで作成
- 変更に強い: 実装の詳細ではなく動作をテスト

### 4. エッジケースを網羅
- 空の入力
- null/undefined/None
- 境界値 (0, 最大値、最小値)
- 無効な入力
- エラー状態

### 5. テストは高速
- インメモリデータベースを使用
- モックを適切に活用
- 重いI/O操作を避ける

## 避けるべきアンチパターン

### フロントエンド
❌ 実装の詳細をテスト
```tsx
// Bad: 内部状態を直接テスト
expect(component.state.count).toBe(1)

// Good: ユーザーから見える動作をテスト
expect(screen.getByText('Count: 1')).toBeInTheDocument()
```

❌ スナップショットテストの過度な使用
```tsx
// Bad: 大きなコンポーネントのスナップショット
expect(container).toMatchSnapshot()

// Good: 特定の動作をテスト
expect(screen.getByRole('button')).toHaveAttribute('disabled')
```

### バックエンド
❌ テスト間で状態を共有
```rust
// Bad: グローバル変数を使用
static mut SHARED_DB: Option<SqlitePool> = None;

// Good: 各テストで独立したDBを作成
let pool = create_test_db().await;
```

❌ 具体的なエラーメッセージに依存
```rust
// Bad: エラーメッセージの完全一致
assert_eq!(err.to_string(), "Connection failed: timeout");

// Good: エラーの種類を確認
assert!(err.to_string().contains("timeout"));
```

## CI/CD での実行

### GitHub Actions の例
```yaml
- name: Run frontend tests
  run: npm run test:frontend:run

- name: Run frontend tests with coverage
  run: npm run test:frontend:coverage

- name: Run backend tests
  run: cd src-tauri && cargo test

- name: Run backend tests with coverage
  run: cd src-tauri && cargo llvm-cov --all-features --workspace
```

## 参考リソース

### フロントエンド
- [Vitest Documentation](https://vitest.dev/)
- [React Testing Library](https://testing-library.com/react)
- [Common Testing Mistakes](https://kentcdodds.com/blog/common-mistakes-with-react-testing-library)

### バックエンド
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [tokio Testing](https://tokio.rs/tokio/topics/testing)
- [sqlx Testing](https://github.com/launchbadge/sqlx#testing)
