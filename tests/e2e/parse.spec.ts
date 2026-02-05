import { test, expect } from './fixtures';
import { navigateToScreen, expectSidebarVisible } from './helpers';

test.describe('Parse画面（Batch内）', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expectSidebarVisible(page);
    await navigateToScreen(page, 'Batch');
  });

  test('パースセクションが表示される', async ({ page }) => {
    await expect(
      page.getByRole('heading', { name: '2. メールパース' })
    ).toBeVisible();
  });

  test('パースを開始ボタンが表示される', async ({ page }) => {
    await expect(
      page.getByRole('button', { name: /パースを開始|パース中/ })
    ).toBeVisible();
  });

  test('確認ダイアログに削除して実行ボタンが表示される', async ({ page }) => {
    const parseButton = page.getByRole('button', {
      name: /パースを開始|パース中/,
    });
    await parseButton.click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(
      page.getByRole('button', { name: '削除して実行' })
    ).toBeVisible();
    await page.getByRole('button', { name: 'キャンセル' }).click();
  });

  test('パースを開始ボタンをクリックすると確認ダイアログが表示される', async ({
    page,
  }) => {
    const parseButton = page.getByRole('button', {
      name: /パースを開始|パース中/,
    });
    await parseButton.click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(
      page.getByRole('heading', { name: 'パース処理の確認' })
    ).toBeVisible();
    await page.getByRole('button', { name: 'キャンセル' }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });
});
