# コードレビュー観点チェック結果

**実施日**: 2026-03-03  
**チェック基準**: [COPILOT_REVIEW_PERSPECTIVES.md](./COPILOT_REVIEW_PERSPECTIVES.md)

---

## チェックサマリ

| カテゴリ             | チェック項目数 | 問題なし | 要改善 | 許容範囲 |
| -------------------- | -------------- | -------- | ------ | -------- |
| セキュリティ         | 5              | 4        | 0      | 1        |
| エラーハンドリング   | 4              | 2        | 0      | 2        |
| 非同期・リソース管理 | 3              | 3        | 0      | 0        |
| React/フロントエンド | 4              | 2        | 2      | 0        |
| コード品質           | 3              | 3        | 0      | 0        |
| **合計**             | **19**         | **14**   | **2**  | **3**    |

---

## 1. セキュリティ

### ✅ 1.1 URL検証（SSRF対策）

**確認箇所**: `src-tauri/src/image_utils.rs`

**結果**: ✅ **問題なし**

- `validate_image_url()` 関数で HTTPS のみ許可
- プライベート IP・localhost ブロック実装済み
- メタデータエンドポイント（169.254.169.254）もブロック
- 画像ダウンロード前に必ず検証を実行

**対応状況**: PR #59 で対応済み

---

### ✅ 1.2 パストラバーサル対策

**確認箇所**:

- `src-tauri/src/metadata/export.rs` (L265)
- `src-tauri/src/metadata/file_safety.rs`
- `src/hooks/useImageUrl.ts` (L59-61)

**結果**: ✅ **問題なし**

- `is_safe_file_name()` 関数で検証実装済み
- エクスポート時に `file_name` を検証してから `join()` に使用
- `useImageUrl.ts` でも `/[/\\]|\.\./` でパストラバーサルを防止
- 不正な `file_name` はスキップされる

**対応状況**: PR #73 で対応済み

---

### ✅ 1.3 画像サイズ・フォーマット制限

**確認箇所**: `src-tauri/src/image_utils.rs`

**結果**: ✅ **問題なし**

- `MAX_IMAGE_SIZE_BYTES = 10MB` の制限あり
- Content-Length と body サイズの両方でチェック
- `image` クレートで JPEG/PNG/WebP のみ許可
- 古い画像ファイルの削除も実装済み

**対応状況**: PR #59 で対応済み

---

### ⚠️ 1.4 assetProtocol スコープ

**確認箇所**: `src-tauri/tauri.conf.json`

**結果**: ⚠️ **許容範囲**（PR #59 で対応済み）

- `$APPDATA/jp.github.hina0118.paa/images/**` に制限済み
- 必要最小限のスコープに設定されている

---

### ✅ 1.5 機密情報のログ出力

**確認箇所**: コードベース全体

**結果**: ✅ **問題なし**

- メール本文・件名のログ出力は適切なレベル（debug/info）に設定
- PR #21 で対応済み

---

## 2. エラーハンドリング・堅牢性

### ⚠️ 2.1 lib.rs の expect() による初期化時のpanic

**確認箇所**: `src-tauri/src/lib.rs` (L162, L172, L179)

**結果**: ⚠️ **許容範囲**

```rust
// L162: DB URL パース
.expect("Failed to parse database URL")

// L172: プール作成
.expect("Failed to create sqlx pool")

// L179: SQLite バージョン取得
.expect("Failed to query SQLite version")
```

**判断理由**:

- アプリ起動時の必須初期化処理
- 失敗時は早期にクラッシュして原因が明確になる
- 運用環境では設定ミスとして検出可能
- 起動時の必須チェックとして許容範囲

**推奨**: ドキュメントに「起動時の必須チェック」として明記

---

### ✅ 2.2 Mutex ロック時の panic 対策

**確認箇所**: `src-tauri/src/lib.rs` (ログバッファ関連)

**結果**: ✅ **問題なし**

- `unwrap()` は使用されていない
- `match` 式で明示的なエラーハンドリング
- PR #21 で対応済み

---

### ✅ 2.3 型変換の安全性

**確認箇所**:

- `src-tauri/src/orchestration/product_parse_orchestrator.rs` (L160)
- `src-tauri/src/orchestration/sync_orchestrator.rs` (L169)
- `src-tauri/src/orchestration/parse_orchestrator.rs` (L93)

**結果**: ✅ **問題なし**

```rust
// clamp で範囲制限後に as usize
let gemini_batch_size = (config.gemini.batch_size.clamp(1, 50)) as usize;
let max_results = (config.sync.max_results_per_page.clamp(1, 500)) as u32;
```

- `clamp()` で範囲制限後に型変換しているため安全
- 極端に大きい値でも範囲内に収まる

---

### ⚠️ 2.4 ウィンドウ設定の型変換

**確認箇所**: `src-tauri/src/lib.rs` (L274-275, L282-283)

**結果**: ⚠️ **許容範囲**

```rust
width: settings.width as u32,
height: settings.height as u32,
x: x_pos as i32,
y: y_pos as i32,
```

**判断理由**:

- ウィンドウサイズ・位置は通常の範囲内（0-32767程度）
- `#[allow(clippy::cast_possible_truncation)]` で意図を明示
- UI から入力される値のため、範囲外の値は来ない想定

---

## 3. 非同期・リソース管理

### ✅ 3.1 setTimeout のクリーンアップ

**確認箇所**:

- `src/components/orders/image-search-dialog.tsx` (L175-179)
- `src/hooks/useDebouncedSearch.ts` (L19-23)
- `src/components/screens/product-master.tsx` (L129-131)
- `src/components/screens/logs.tsx` (L53-57)
- `src/App.tsx` (L152-156)

**結果**: ✅ **問題なし**

- すべての `setTimeout` で `useEffect` の return で `clearTimeout` を実行
- `setInterval` も `clearInterval` でクリーンアップ
- `App.tsx` の `saveTimeout` も `debouncedSave` 内で適切にクリーンアップ

---

### ✅ 3.2 Promise の未処理 rejection

**確認箇所**: `src/contexts/sync-provider.tsx`, `src/contexts/parse-provider.tsx`

**結果**: ✅ **問題なし**

- PR #74 で対応済み
- `notify()` は適切に `await` または `void` で処理

---

### ✅ 3.3 古いリソースの削除

**確認箇所**: `src-tauri/src/image_utils.rs` (L266-272)

**結果**: ✅ **問題なし**

- 画像更新時に古い `file_name` のファイルを削除
- エラー時も `log::warn!` で記録して継続

---

## 4. React / フロントエンド

### ⚠️ 4.1 key に index を使用

**確認箇所**: `src/components/screens/shop-settings.tsx` (L315, L543)

**結果**: ⚠️ **要改善（P2）**

```tsx
// L315: 件名フィルターの動的リスト
{
  newSubjectFilters.map((filter, index) => (
    <div key={index} className="flex flex-col gap-1">
      ...
    </div>
  ));
}

// L543: 編集フォームの件名フィルター
{
  (editForm.subject_filters_array || ['']).map((filter, index) => (
    <div key={index} className="flex flex-col gap-1">
      ...
    </div>
  ));
}
```

**問題点**:

- フィルターの追加・削除・並び替え時に React の再レンダリングが不正確になる可能性
- 一意の ID がないため、`key` に `index` を使用

**推奨対応**:

- フィルターに一意の ID を付与（UUID または `filter-${index}-${filter}` 形式）
- または、フィルターの内容が変更されない前提なら現状維持も可

**優先度**: P2（Nitpick）

---

### ✅ 4.2 onError 無限ループ対策

**確認箇所**: `src/components/orders/image-search-dialog.tsx`

**結果**: ✅ **問題なし**

- PR #59 で `data-error-handled` 属性による対策済み

---

### ✅ 4.3 アクセシビリティ（aria-label）

**確認箇所**: コードベース全体

**結果**: ✅ **問題なし**

- PR #59, #74 で対応済み
- ボタンに適切な `aria-label` が設定されている

---

### ✅ 4.4 useEffect の依存配列

**確認箇所**: コードベース全体

**結果**: ✅ **問題なし**

- PR #59 で依存配列の不適切な使用を修正済み

---

## 5. コード品質

### ✅ 5.1 API 一貫性

**確認箇所**: Tauri コマンドの戻り値型

**結果**: ✅ **問題なし**

- PR #60 で `Result<T, E>` に統一済み

---

### ✅ 5.2 ロジックの重複

**確認箇所**: コードベース全体

**結果**: ✅ **問題なし**

- PR #62 で `getProductMetadata` を utils に抽出済み

---

### ✅ 5.3 未使用インポート

**確認箇所**: コードベース全体

**結果**: ✅ **問題なし**

- PR #59 で対応済み

---

## 6. データベース・マイグレーション

### ✅ 6.1 マイグレーションの整合性

**確認箇所**: `src-tauri/migrations/`

**結果**: ✅ **問題なし**

- PR #62 で 001 への集約が完了
- 既存 DB への適用方針もドキュメント化済み

---

## 7. テスト

### ✅ 7.1 テストカバレッジ

**確認箇所**: テストファイル全体

**結果**: ✅ **問題なし**

- 主要機能にテストが追加されている
- PR #59, #60, #62, #73, #74 で対応済み

---

## 総評

### 良い点

1. **セキュリティ対策が充実**: SSRF、パストラバーサル、リソース制限が適切に実装されている
2. **エラーハンドリング**: 過去の PR で主要な問題は対応済み
3. **非同期処理**: setTimeout/setInterval のクリーンアップが適切
4. **コード品質**: 重複ロジックの抽出、API 一貫性が保たれている

### 改善推奨（優先度順）

| 優先度 | 項目                              | ファイル            | 行            | 対応方針                                   |
| ------ | --------------------------------- | ------------------- | ------------- | ------------------------------------------ |
| **P2** | key={index} の使用                | `shop-settings.tsx` | 315, 543      | フィルターに一意 ID を付与、または現状維持 |
| **P2** | lib.rs の expect() ドキュメント化 | `lib.rs`            | 162, 172, 179 | 起動時の必須チェックとしてコメント追加     |

### 対応不要（許容範囲）

1. **lib.rs の expect()**: 起動時の必須チェックとして許容
2. **ウィンドウ設定の型変換**: 通常の範囲内で問題なし
3. **assetProtocol スコープ**: PR #59 で適切に制限済み

---

## 次のアクション

1. **即時対応不要**: 現状のコード品質は高く、重大な問題は見つかっていない
2. **任意対応**: `shop-settings.tsx` の `key={index}` を改善（P2）
3. **ドキュメント化**: `lib.rs` の `expect()` にコメント追加（P2）

---

## 参考

- [COPILOT_REVIEW_PERSPECTIVES.md](./COPILOT_REVIEW_PERSPECTIVES.md) - レビュー観点まとめ
- [CODE_REVIEW.md](./CODE_REVIEW.md) - コードベース全体レビュー
- [security-fixes/pr21-review-analysis.md](./security-fixes/pr21-review-analysis.md) - PR #21 脅威度分析
