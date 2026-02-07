# PR #74 レビューコメント対応計画

**PR**: [#74 feat: トースト通知の共通化とデスクトップ通知の切り替え](https://github.com/hina0118/paa/pull/74)  
**作成日**: 2026-02-07  
**ブランチ**: `toast`  
**CI ステータス**: pending

---

## 概要

PR #74 は通知の一貫性向上のため、トースト通知の共通化とアプリ非表示時のデスクトップ通知切り替えを実装しています。

### 主な変更

1. **トースト共通ユーティリティ** (`src/lib/toast.ts`)：`formatError`, `toastSuccess`, `toastError`, `toastWarning`, `toastInfo`
2. **各画面のインライン通知をトーストに統一**：Backup, Dashboard, Orders, Batch, Settings, API Keys, Shop Settings, Logs, Tables, Image Search Dialog
3. **バッチ処理完了時の通知切り替え**：アプリ表示中 → トースト、アプリ非表示 → デスクトップ通知
4. **テスト対応**：plugin-notification モック、E2E ヘルパー（expectSuccessMessage / expectErrorMessage）のトースト対応、`src/lib/toast.test.ts` 追加

---

## 未対応レビューコメント一覧

| #   | 優先度            | ファイル                          | 行  | 指摘内容                                                                                                                                                                                                      | 対応方針                                                        |
| --- | ----------------- | --------------------------------- | --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| 1   | **P1: Important** | `src/test/setup.ts`               | 77  | setup で `@tauri-apps/plugin-notification` を一律モックしているが、`utils.test.ts` では個別にモックして呼び出し回数などを検証。Vitest のモック解決順で競合し、utils.test のアサーションが成立しなくなる可能性 | setup 側でモックを export してテストから参照できるようにする    |
| 2   | **P1: Important** | `src/contexts/sync-provider.tsx`  | 64  | `notify` は Promise を返すのに await されていない。未処理の Promise rejection になる可能性                                                                                                                    | `await notify(...)` または `void notify(...).catch(...)` で明示 |
| 3   | **P1: Important** | `src/contexts/parse-provider.tsx` | 78  | 同上、`notify` を await していない                                                                                                                                                                            | 同上                                                            |
| 4   | **P1: Important** | `src/contexts/parse-provider.tsx` | 107 | 同上、`notify` を await していない                                                                                                                                                                            | 同上                                                            |
| 5   | **P1: Important** | `tests/e2e/helpers.ts`            | 45  | `message` 指定時の locator が複数要素にマッチし得る。Playwright の `toBeVisible()` は strict mode で複数一致だと失敗することがある                                                                            | `.first()` を付けるか、件数を絞る条件を追加                     |
| 6   | **P1: Important** | `tests/e2e/helpers.ts`            | 64  | 同上、`message` 指定時の locator の複数一致問題                                                                                                                                                               | 同上                                                            |

**全6件が P1（Important）**

---

## 対応計画

### Phase 1: テストモックの競合解消（P1）

#### 1-1. setup.ts で notification モックを export する

**ファイル**: `src/test/setup.ts`  
**問題**: 71–74 行で `vi.mock('@tauri-apps/plugin-notification', ...)` により一律モックしているが、`utils.test.ts` は同モジュールを個別にモックして `mockIsPermissionGranted` 等の呼び出し回数を検証している。setup のモックが優先されると utils.test のアサーションが成立しなくなる。

**対応方針**:

1. setup 側で notification のモックを変数として定義し、export する
2. `utils.test.ts` では setup から export されたモックを import して使用する（utils.test 側の `vi.mock` を削除）

**修正案**:

```ts
// src/test/setup.ts
const mockNotificationIsPermissionGranted = vi.fn().mockResolvedValue(true);
const mockNotificationRequestPermission = vi.fn().mockResolvedValue('granted');
const mockNotificationSendNotification = vi.fn().mockResolvedValue(undefined);

vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: mockNotificationIsPermissionGranted,
  requestPermission: mockNotificationRequestPermission,
  sendNotification: mockNotificationSendNotification,
}));

// Export mocks for use in tests
export {
  mockInvoke,
  mockListen,
  mockEmit,
  mockNotificationIsPermissionGranted,
  mockNotificationRequestPermission,
  mockNotificationSendNotification,
};
```

**utils.test.ts** の変更:

- `vi.mock('@tauri-apps/plugin-notification', ...)` を削除
- setup から `mockNotificationIsPermissionGranted`, `mockNotificationRequestPermission`, `mockNotificationSendNotification` を import
- 変数名を `mockIsPermissionGranted` 等から setup の名前に合わせる、または setup の export をそのまま使用

---

### Phase 2: notify の Promise 未処理解消（P1）

#### 2-1. sync-provider.tsx の notify 呼び出し

**ファイル**: `src/contexts/sync-provider.tsx`  
**場所**: 56–65 行付近（`else` ブロック内の `notify` 2箇所）

**修正**: `notify` を `await` する。イベントハンドラ内で async なので、`await notify(...)` で問題なし。失敗時は `try/catch` で `console.error` にログ出力。

```ts
} else {
  if (data.error) {
    try {
      await notify('Gmail同期失敗', data.error);
    } catch (error) {
      console.error('Failed to send Gmail sync failure notification:', error);
    }
  } else {
    const body =
      data.success_count > 0
        ? `新たに${data.success_count}件のメールを取り込みました`
        : '新規メッセージはありませんでした';
    try {
      await notify('Gmail同期完了', body);
    } catch (error) {
      console.error('Failed to send Gmail sync completion notification:', error);
    }
  }
}
```

#### 2-2. parse-provider.tsx の notify 呼び出し（2箇所）

**ファイル**: `src/contexts/parse-provider.tsx`  
**場所**: 71–78 行（メールパース完了時）、99–107 行（商品名解析完了時）

**修正**: 同様に `await notify(...)` と `try/catch` を追加。

---

### Phase 3: E2E ヘルパーの strict mode 対策（P1）

#### 3-1. expectSuccessMessage / expectErrorMessage に .first() を付与

**ファイル**: `tests/e2e/helpers.ts`  
**場所**: 33–45 行（expectSuccessMessage）、52–64 行（expectErrorMessage）

**問題**: `message` 指定時に `.filter({ hasText: message })` で複数要素にマッチし得る。Playwright の `expect(locator).toBeVisible()` は strict mode で複数一致だと失敗する。

**修正**: `message` 指定時も `.first()` で単一要素にする。

```ts
export async function expectSuccessMessage(page: Page, message?: string) {
  const toastOrInline = message
    ? page
        .locator(
          '[data-sonner-toast], [data-testid="success-message"], [role="status"]'
        )
        .filter({ hasText: message })
        .first()
    : page
        .locator(
          '[data-sonner-toast], [data-testid="success-message"], [role="status"]'
        )
        .first();
  await expect(toastOrInline).toBeVisible({ timeout: 10000 });
}

export async function expectErrorMessage(page: Page, message?: string) {
  const toastOrInline = message
    ? page
        .locator(
          '[data-sonner-toast], [data-testid="error-message"], [role="alert"]'
        )
        .filter({ hasText: message })
        .first()
    : page
        .locator(
          '[data-sonner-toast], [data-testid="error-message"], [role="alert"]'
        )
        .first();
  await expect(toastOrInline).toBeVisible({ timeout: 10000 });
}
```

---

## 対応順序

1. **Phase 1**: setup.ts の notification モック export と utils.test.ts の修正
2. **Phase 2**: sync-provider.tsx、parse-provider.tsx の notify await 対応
3. **Phase 3**: tests/e2e/helpers.ts の .first() 追加

---

## 完了チェックリスト

- [ ] Phase 1: setup.ts のモック export、utils.test.ts の修正
- [ ] Phase 2: sync-provider.tsx の notify await
- [ ] Phase 2: parse-provider.tsx の notify await（2箇所）
- [ ] Phase 3: helpers.ts の expectSuccessMessage / expectErrorMessage に .first() 追加
- [ ] `npm run test` 成功
- [ ] `npm run lint` 成功
- [ ] E2E テスト成功（必要に応じて）
- [ ] プッシュ後 CI 成功
- [ ] レビューコメントを解決済みにする
