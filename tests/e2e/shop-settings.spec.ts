import { test, expect } from './fixtures';
import { navigateToScreen, expectSidebarVisible } from './helpers';

test.describe('Shop Settings画面', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expectSidebarVisible(page);
    await navigateToScreen(page, 'Shop Settings');
  });

  test('Shop Settings画面が表示される', async ({ page }) => {
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  });
});
