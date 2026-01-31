# コードベース全体レビュー

実施日: 2026-01-31

## Vitest セットアップ問題（解消済み）

**現象**: `Vitest failed to find the runner` または `No test suite found in file`

**原因**: Windows でドライブ文字が小文字（`c:\`）のとき、Vitest 4 のパス解決で不具合が発生する。

**対処**: プロジェクトへ `cd` する際に大文字ドライブ（`C:\`）を使用する。詳細は `TESTING_FRONTEND.md` のトラブルシューティングを参照。

## 1. アーキテクチャ・構造

### 良い点

- **責務の分離**: `lib/`（DB・クエリ・ユーティリティ）、`hooks/`、`contexts/`、`components/` が整理されている
- **テーブル名のホワイトリスト**: `table-utils.ts` の `VALID_TABLES` による SQL インジェクション対策
- **DatabaseManager の設計**: シングルトン、初期化競合の考慮、クリーンアップ処理が明確
- **E2E モック**: Tauri 非稼働時のフォールバック（`e2e-mock-db.ts`）が設計されている

### 改善余地

- **App.tsx の責務**: DB 初期化・ウィンドウ設定・通知リスナーが一つの `useEffect` に集約されている。関心ごとに分割すると可読性・テスト性が上がる

---

## 2. セキュリティ

### 良い点

- **テーブル名のサニタイズ**: `sanitizeTableName()` によるホワイトリスト検証
- **パストラバーサル防止**: `useImageUrl` で `/[\/\\]|\.\./` を拒否
- **プレースホルダー利用**: `orders-queries.ts` の検索・フィルタはバインドパラメータ使用
- **既存のセキュリティ修正**: `docs/security-fixes/` にログ・Base64・Mutex 等の対策が記録されている

### 確認事項

- **PRAGMA のテーブル名**: `table-viewer.tsx` は `sanitizeTableName()` 後に `safeTableName` を文字列補間している。ホワイトリスト通過後のみ使用されるため問題なし

---

## 3. コード品質・ベストプラクティス

### 良い点

- **型定義**: `OrderItemRow`、`LoadParams` など型が明確
- **parseNumericFilter**: `orders.tsx` で NaN を防ぐ正規化を実施
- **loadItems の競合対策**: `requestId` による古いレスポンスの破棄
- **ESLint 設定**: React Hooks、`no-console`、Prettier 連携が適切

### 改善提案

#### 3.1 App.tsx: 非同期クリーンアップのタイミング

```tsx
// 現状: setupWindowListeners().then() で cleanup を設定するが、
// アンマウント時に promise が未解決だと cleanup が undefined のまま
let cleanup: (() => void) | undefined;
setupWindowListeners().then((fn) => {
  cleanup = fn;
});
return () => {
  if (cleanup) cleanup();  // promise 未解決時は呼ばれない
  ...
};
```

**提案**: `useRef` で cleanup を保持し、クリーンアップ時に `ref.current?.()` を呼ぶ。または、`setupWindowListeners` の解決を待ってからリスナー登録するなど、クリーンアップが確実に登録される設計にする。

#### 3.2 sync-context.tsx: refreshStatus の依存関係

`sync-progress` リスナー内で `refreshStatus()` を呼んでいるが、`useEffect` の依存配列に含まれていない。`refreshStatus` を `useCallback` でラップし、必要なら依存配列に含めるか、`useRef` で最新の `refreshStatus` を参照する。

#### 3.3 Settings: 成功/エラーメッセージの競合

`successMessage` と `errorMessage` を別々に管理しているため、両方表示される可能性がある。保存時に `setErrorMessage('')` しているが、逆のケース（成功後にエラー）の考慮は要確認。

---

## 4. パフォーマンス

### 良い点

- **deliveryStatus クエリ**: 相関サブクエリから CTE + JOIN に変更済み
- **deliveries インデックス**: `idx_deliveries_order_id_updated_at` の追加
- **検索デバウンス**: Orders の検索入力で 300ms デバウンス
- **仮想スクロール**: `@tanstack/react-virtual` で大量アイテムを効率表示

### 確認事項

- **loadFilters / loadItems**: 両方 `useEffect` で実行。`loadFilters` は初回のみで十分なら、依存配列の見直しで不要な再実行を減らせる可能性あり

---

## 5. テスト

### 良い点

- **フロントエンド**: 93.60% カバレッジ、主要コンポーネントをカバー
- **useImageUrl**: `resetImageUrlCacheForTests` で `resetModules` を避けた設計
- **orders-queries**: モック DB による単体テスト
- **E2E**: Playwright と WDIO（Tauri）の両方を用意

### 改善余地

- **table-viewer**: テーブル表示・ページネーションのテストが不足している可能性
- **DatabaseManager**: シングルトン・クリーンアップの統合テストがあると安心

---

## 6. その他の指摘

### 6.1 orders-queries.ts: sortBy / sortOrder

`sortBy` と `sortOrder` はユーザー入力由来だが、`orderCol` / `orderDir` は固定の列名・方向のみを許容している。現状の UI からは問題ないが、将来外部入力になる場合は検証が必要。

### 6.2 handleClearFilters と searchDebounced

`handleClearFilters` で `setSearchDebounced('')` を直接呼んでいるため、クリア時はデバウンスをバイパスして即時反映される。意図通りで問題なし。

### 6.3 TableViewer: key={index}

```tsx
data.map((row, index) => (
  <TableRow key={index}>
```

行に一意の ID があれば `key={row.id}` の方が望ましい。スキーマによっては `id` が無いテーブルもあるため、現状は許容範囲。

---

## 7. 推奨アクション（優先度順）

| 優先度 | 項目                                                 | 工数 | 状態    |
| ------ | ---------------------------------------------------- | ---- | ------- |
| 高     | App.tsx のウィンドウリスナー・クリーンアップの確実性 | 小   | ✅ 完了 |
| 中     | sync-context の refreshStatus を useCallback 化      | 小   | ✅ 完了 |
| 中     | TableViewer の key を id ベースに変更（可能な場合）  | 小   | ✅ 完了 |
| 低     | App.tsx の useEffect を関心ごとに分割                | 中   | ✅ 完了 |
| 低     | DatabaseManager の統合テスト追加                     | 中   | ✅ 完了 |

---

## 8. 総評

- アーキテクチャ、セキュリティ、テストの基盤は整っている
- 最近の修正（deliveryStatus クエリ、parseNumericFilter、useImageUrl テスト）で品質が向上している
- 上記の改善点は小〜中規模で、段階的に対応可能
