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
    await expect(page.getByRole('button', { name: 'Sync' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Parse' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Logs' })).toBeVisible();
    await expect(
      page.getByRole('button', { name: 'Shop Settings' })
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
    // Orders画面のタイトルを確認（EmailListコンポーネントのタイトル）
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  });

  test('Sync画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'Sync');
    await expectScreenTitle(page, 'Gmail同期');
  });

  test('Parse画面に遷移できる', async ({ page }) => {
    await navigateToScreen(page, 'Parse');
    // Parse画面のタイトルを確認
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
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

  test('アクティブな画面のボタンがハイライトされる', async ({ page }) => {
    // 初期状態はOrdersがアクティブ
    const ordersButton = page.getByRole('button', { name: 'Orders' });
    // variant="secondary"のボタンは`bg-secondary`クラスを持つ
    const ordersButtonClasses = await ordersButton.getAttribute('class');
    expect(ordersButtonClasses).toContain('bg-secondary');

    // Dashboardに遷移
    await navigateToScreen(page, 'Dashboard');
    const dashboardButton = page.getByRole('button', { name: 'Dashboard' });
    const dashboardButtonClasses = await dashboardButton.getAttribute('class');
    expect(dashboardButtonClasses).toContain('bg-secondary');

    // Ordersは非アクティブになる（ghost variantは`bg-secondary`を持たない）
    const ordersButtonClassesAfter = await ordersButton.getAttribute('class');
    expect(ordersButtonClassesAfter).not.toContain('bg-secondary');
  });
});
