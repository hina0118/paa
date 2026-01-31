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
      page.getByRole('button', { name: 'ログを手動で更新' })
    ).toBeVisible();
  });

  test('更新ボタンをクリックできる', async ({ page }) => {
    const updateButton = page.getByRole('button', {
      name: 'ログを手動で更新',
    });
    await updateButton.click();
    await expect(updateButton).toBeVisible();
  });

  test('自動更新ボタンをトグルできる', async ({ page }) => {
    const autoRefreshButton = page.getByRole('button', {
      name: '自動更新を開始',
    });
    await expect(autoRefreshButton).toBeVisible();
    await autoRefreshButton.click();
    await expect(
      page.getByRole('button', { name: '自動更新を停止' })
    ).toBeVisible();
  });

  test('ログ検索に入力できる', async ({ page }) => {
    const searchInput = page.getByPlaceholder('ログメッセージを検索...');
    await expect(searchInput).toBeVisible();
    await searchInput.fill('test');
    await expect(searchInput).toHaveValue('test');
  });

  test('ログレベルでフィルタできる', async ({ page }) => {
    const infoFilterButton = page.getByRole('button', {
      name: 'INFOレベルのログでフィルタ',
    });
    await expect(infoFilterButton).toBeVisible();
    await infoFilterButton.click();
    await expect(
      page.getByRole('button', { name: 'INFOレベルのログフィルタを解除' })
    ).toBeVisible();
  });

  test('フィルタ設定後に全てクリアで解除できる', async ({ page }) => {
    const infoFilterButton = page.getByRole('button', {
      name: 'INFOレベルのログでフィルタ',
    });
    await infoFilterButton.click();
    const clearAllButton = page.getByRole('button', { name: '全てクリア' });
    await expect(clearAllButton).toBeVisible();
    await clearAllButton.click();
    await expect(
      page.getByRole('button', { name: 'INFOレベルのログでフィルタ' })
    ).toBeVisible();
  });
});
