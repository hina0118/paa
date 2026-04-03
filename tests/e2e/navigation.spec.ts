import { test, expect } from './fixtures';
import {
  navigateToScreen,
  expectScreenTitle,
  expectSidebarVisible,
  dismissToasts,
} from './helpers';

test.describe('ナビゲーション', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // サイドバーが表示されるまで待機
    await expectSidebarVisible(page);
    await dismissToasts(page);
  });

  test('サイドバーが表示される', async ({ page }) => {
    await expectSidebarVisible(page);
    // ナビゲーション項目が表示されることを確認
    await expect(page.getByTestId('orders')).toBeVisible();
    await expect(page.getByTestId('batch')).toBeVisible();
    await expect(page.getByTestId('logs')).toBeVisible();
    await expect(page.getByTestId('shop-settings')).toBeVisible();
    await expect(page.getByTestId('api-keys')).toBeVisible();
    await expect(page.getByTestId('settings')).toBeVisible();
  });

  test('Orders画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'orders');
    await expectScreenTitle(page, '商品一覧');
  });

  test('Batch画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'batch');
    await expectScreenTitle(page, 'バッチ処理');
  });

  test('Logs画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'logs');
    // Logs画面のタイトルを確認
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  });

  test('Shop Settings画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'shop-settings');
    // Shop Settings画面のタイトルを確認
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  });

  test('Settings画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'settings');
    await expectScreenTitle(page, '設定');
  });

  test('Tablesセクションを展開して閉じることができる', async ({ page }) => {
    const tablesButton = page.getByTestId('tables-section-toggle');
    await tablesButton.click();
    await expect(page.getByTestId('table-emails')).toBeVisible();
    await tablesButton.click();
    await expect(page.getByTestId('table-emails')).not.toBeVisible();
  });

  test('アクティブな画面のボタンがハイライトされる', async ({ page }) => {
    // 初期状態はOrdersがアクティブ（デフォルト画面）
    const ordersButton = page.getByTestId('orders');
    await expect(ordersButton).toHaveAttribute('aria-current', 'page');

    // Batchに遷移
    await navigateToScreen(page, 'batch');
    const batchButton = page.getByTestId('batch');
    await expect(batchButton).toHaveAttribute('aria-current', 'page');

    // Ordersは非アクティブになる（aria-current が付かない）
    await expect(ordersButton).not.toHaveAttribute('aria-current');
  });
});
