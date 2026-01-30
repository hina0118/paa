import { test, expect } from './fixtures';
import { navigateToScreen, expectSidebarVisible } from './helpers';

test.describe('Logs画面', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expectSidebarVisible(page);
    await navigateToScreen(page, 'Logs');
  });

  test('Logs画面が表示される', async ({ page }) => {
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  });

  test('更新ボタンが表示される', async ({ page }) => {
    await expect(
      page.getByRole('button', { name: /更新|読み込み中/ })
    ).toBeVisible();
  });
});
