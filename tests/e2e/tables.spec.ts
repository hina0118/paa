import { test, expect } from './fixtures';
import type { Page } from '@playwright/test';
import { expectSidebarVisible } from './helpers';

function expandTablesAndNavigate(page: Page, tableId: string) {
  return async () => {
    const tablesButton = page.getByTestId('tables-section-toggle');
    await tablesButton.click();
    await page.getByTestId(tableId).click();
  };
}

test.describe('Tables画面', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expectSidebarVisible(page);
  });

  test('Tablesセクションを展開してEmailsテーブルに遷移できる', async ({
    page,
  }) => {
    await expandTablesAndNavigate(page, 'table-emails')();
    await expect(
      page.getByRole('heading', { name: /Emails テーブル/ })
    ).toBeVisible();
  });

  test('複数のテーブルに遷移できる', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-emails')();
    await expect(
      page.getByRole('heading', { name: /Emails テーブル/ })
    ).toBeVisible({ timeout: 5000 });

    const tables = [
      { id: 'table-orders', heading: 'Orders' },
      { id: 'table-items', heading: 'Items' },
      { id: 'table-images', heading: 'Images' },
      { id: 'table-deliveries', heading: 'Deliveries' },
      { id: 'table-htmls', heading: 'HTMLs' },
      { id: 'table-order-emails', heading: 'Order-Emails' },
      { id: 'table-order-htmls', heading: 'Order-HTMLs' },
      { id: 'table-shop-settings', heading: 'Shop Settings' },
      { id: 'table-product-master', heading: 'Product Master' },
      { id: 'table-item-overrides', heading: 'Item Overrides' },
      { id: 'table-order-overrides', heading: 'Order Overrides' },
      { id: 'table-excluded-items', heading: 'Excluded Items' },
      { id: 'table-excluded-orders', heading: 'Excluded Orders' },
    ] as const;
    for (const table of tables) {
      await page.getByTestId(table.id).click();
      await expect(
        page.getByRole('heading', { name: new RegExp(`${table.heading} テーブル`) })
      ).toBeVisible({ timeout: 5000 });
    }
  });

  test('EmailsからOrdersテーブルに遷移できる', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-emails')();
    await expect(
      page.getByRole('heading', { name: /Emails テーブル/ })
    ).toBeVisible({ timeout: 5000 });
    await page.getByTestId('table-orders').click();
    await expect(
      page.getByRole('heading', { name: /Orders テーブル/ })
    ).toBeVisible({ timeout: 5000 });
  });

  test('テーブル画面で読み込み中または結果が表示される', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-emails')();
    await expect(
      page.getByRole('heading', { name: /Emails テーブル/ })
    ).toBeVisible();
    await expect(
      page
        .getByText('読み込み中')
        .or(page.getByText(/エラー/))
        .or(page.getByText(/件を表示|データがありません/))
    ).toBeVisible({ timeout: 10000 });
  });

  test('更新ボタンをクリックできる', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-emails')();
    await expect(
      page.getByRole('heading', { name: /Emails テーブル/ })
    ).toBeVisible();
    const refreshButton = page.getByRole('button', { name: '更新' });
    await expect(refreshButton).toBeVisible({ timeout: 10000 });
    await refreshButton.click();
  });

  test('ページネーションで次へをクリックできる', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-shop-settings')();
    await expect(
      page.getByRole('heading', { name: /Shop Settings テーブル/ })
    ).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(/55件/)).toBeVisible();
    const nextButton = page.getByRole('button', { name: /次へ/ });
    await expect(nextButton).toBeEnabled();
    await nextButton.click();
    await expect(page.getByText(/ページ 2/)).toBeVisible();
  });

  test('前へ・次へボタンが表示される', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-emails')();
    await expect(
      page.getByRole('heading', { name: /Emails テーブル/ })
    ).toBeVisible();
    await expect(page.getByRole('button', { name: /前へ/ })).toBeVisible({
      timeout: 10000,
    });
    await expect(page.getByRole('button', { name: /次へ/ })).toBeVisible();
  });

  test('セルをクリックすると詳細ダイアログが表示される', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-shop-settings')();
    await expect(
      page.getByRole('heading', { name: /Shop Settings テーブル/ })
    ).toBeVisible({ timeout: 10000 });
    const clickableCell = page
      .locator('td[title="クリックして全文表示"]')
      .first();
    await expect(clickableCell).toBeVisible({ timeout: 5000 });
    await clickableCell.click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByText('セルの全内容')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });
});
