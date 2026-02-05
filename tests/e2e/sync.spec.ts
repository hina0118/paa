import { test, expect } from './fixtures';
import {
  navigateToScreen,
  expectScreenTitle,
  expectSidebarVisible,
} from './helpers';

test.describe('Sync画面（Batch内）', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expectSidebarVisible(page);
    await navigateToScreen(page, 'Batch');
    await expectScreenTitle(page, 'バッチ処理');
  });

  test('Gmail同期セクションが表示される', async ({ page }) => {
    await expect(
      page.getByRole('heading', { name: '1. Gmail同期' })
    ).toBeVisible();
  });

  test('同期開始ボタンがクリックできる', async ({ page }) => {
    const startButton = page.getByRole('button', { name: '同期を開始' });
    await expect(startButton).toBeVisible();
    await startButton.click();
    // Tauri API がなくてもクリックは実行される（エラー表示になる可能性あり）
    await expect(
      page.getByRole('button', { name: /同期を開始|同期中\.\.\.|同期を再開/ })
    ).toBeVisible();
  });
});
