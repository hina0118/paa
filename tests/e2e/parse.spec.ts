import { test, expect } from './fixtures';
import { navigateToScreen, expectSidebarVisible } from './helpers';

test.describe('Parse画面', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expectSidebarVisible(page);
    await navigateToScreen(page, 'Parse');
  });

  test('Parse画面が表示される', async ({ page }) => {
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  });

  test('パースを開始ボタンが表示される', async ({ page }) => {
    await expect(
      page.getByRole('button', { name: /パースを開始|パース中/ })
    ).toBeVisible();
  });
});
