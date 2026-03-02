import { test, expect } from './fixtures';
import {
  navigateToScreen,
  expectScreenTitle,
  expectSidebarVisible,
  dismissToasts,
} from './helpers';

test.describe('Sync画面（Batch内）', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expectSidebarVisible(page);
    await navigateToScreen(page, 'batch');
    await expectScreenTitle(page, 'バッチ処理');
    await dismissToasts(page);
  });

  test('Gmail同期セクションが表示される', async ({ page }) => {
    await expect(
      page.getByRole('heading', { name: '1. Gmail同期' })
    ).toBeVisible();
  });

  test('差分同期ボタンがクリックできる', async ({ page }) => {
    const startButton = page.getByRole('button', { name: '差分同期' });
    await expect(startButton).toBeVisible();
    await startButton.click();
    await expect(
      page.getByRole('button', { name: /差分同期|同期中\.\.\./ })
    ).toBeVisible();
  });

  test('全件同期ボタンが表示される', async ({ page }) => {
    const fullSyncButton = page.getByRole('button', { name: '全件同期' });
    await expect(fullSyncButton).toBeVisible();
  });
});
