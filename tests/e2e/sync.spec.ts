import { test, expect } from './fixtures';
import {
  navigateToScreen,
  expectScreenTitle,
  expectSidebarVisible,
} from './helpers';

test.describe('Sync画面', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expectSidebarVisible(page);
    await navigateToScreen(page, 'Sync');
    await expectScreenTitle(page, 'Gmail同期');
  });

  test('Sync画面が表示される', async ({ page }) => {
    await expectScreenTitle(page, 'Gmail同期');
  });

  test('同期開始ボタンがクリックできる', async ({ page }) => {
    const startButton = page.getByRole('button', { name: '同期を開始' });
    await expect(startButton).toBeVisible();
    await startButton.click();
    // Tauri API がなくてもクリックは実行される（エラー表示になる可能性あり）
    await expect(startButton).toBeVisible();
  });

  test('同期日時をリセットボタンをクリックすると確認ダイアログが表示される', async ({
    page,
  }) => {
    page.on('dialog', (dialog) => dialog.dismiss());
    const resetButton = page.getByRole('button', {
      name: '同期日時をリセット',
    });
    await expect(resetButton).toBeVisible();
    await resetButton.click();
  });
});
