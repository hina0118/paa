import { test, expect } from './fixtures';
import {
  navigateToScreen,
  expectSidebarVisible,
  expectScreenTitle,
} from './helpers';

test.describe('Shop Settings画面', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expectSidebarVisible(page);
    await navigateToScreen(page, 'Shop Settings');
  });

  test('Shop Settings画面が表示される', async ({ page }) => {
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  });

  test('店舗追加フォームに入力できる', async ({ page }) => {
    await expectScreenTitle(page, '店舗設定');
    const shopNameInput = page.getByPlaceholder('例: Amazon発送通知').first();
    await shopNameInput.waitFor({ state: 'visible', timeout: 5000 });
    await shopNameInput.fill('Test Shop');
    await expect(shopNameInput).toHaveValue('Test Shop');
  });

  test('空の状態で追加ボタンをクリックするとバリデーションエラーが表示される', async ({
    page,
  }) => {
    const addButton = page.getByRole('button', { name: '追加', exact: true });
    await addButton.waitFor({ state: 'visible', timeout: 5000 });
    await addButton.click();
    await expect(
      page.getByText('すべての項目を入力してください')
    ).toBeVisible();
  });
});
