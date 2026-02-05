import { test, expect } from './fixtures';
import {
  navigateToScreen,
  expectScreenTitle,
  expectSidebarVisible,
} from './helpers';

test.describe('ナビゲーション', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // サイドバーが表示されるまで待機
    await expectSidebarVisible(page);
  });

  test('サイドバーが表示される', async ({ page }) => {
    await expectSidebarVisible(page);
    // ナビゲーション項目が表示されることを確認
    await expect(page.getByRole('button', { name: 'Dashboard' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Orders' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Batch' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Logs' })).toBeVisible();
    await expect(
      page.getByRole('button', { name: 'Shop Settings' })
    ).toBeVisible();
    await expect(
      page.getByRole('button', { name: 'API Keys', exact: true })
    ).toBeVisible();
    // "Settings"はexact matchを使用（"Shop Settings"と区別するため）
    await expect(
      page.getByRole('button', { name: 'Settings', exact: true })
    ).toBeVisible();
  });

  test('Dashboard画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'Dashboard');
    await expectScreenTitle(page, 'ダッシュボード');
  });

  test('Orders画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'Orders');
    await expectScreenTitle(page, '商品一覧');
  });

  test('Batch画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'Batch');
    await expectScreenTitle(page, 'バッチ処理');
  });

  test('Logs画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'Logs');
    // Logs画面のタイトルを確認
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  });

  test('Shop Settings画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'Shop Settings');
    // Shop Settings画面のタイトルを確認
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  });

  test('Settings画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'Settings');
    await expectScreenTitle(page, '設定');
  });

  test('Tablesセクションを展開して閉じることができる', async ({ page }) => {
    const tablesButton = page.getByRole('button', { name: /Tables/ });
    await tablesButton.click();
    await expect(
      page.getByRole('button', { name: 'Emails', exact: true })
    ).toBeVisible();
    await tablesButton.click();
    await expect(
      page.getByRole('button', { name: 'Emails', exact: true })
    ).not.toBeVisible();
  });

  test('アクティブな画面のボタンがハイライトされる', async ({ page }) => {
    // 初期状態はOrdersがアクティブ（デフォルト画面）
    const ordersButton = page.getByRole('button', { name: 'Orders' });
    await expect(ordersButton).toHaveAttribute('aria-current', 'page');

    // Dashboardに遷移
    await navigateToScreen(page, 'Dashboard');
    const dashboardButton = page.getByRole('button', { name: 'Dashboard' });
    await expect(dashboardButton).toHaveAttribute('aria-current', 'page');

    // Ordersは非アクティブになる（aria-current が付かない）
    await expect(ordersButton).not.toHaveAttribute('aria-current');
  });
});
