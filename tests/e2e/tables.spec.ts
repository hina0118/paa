import { test, expect } from './fixtures';
import type { Page } from '@playwright/test';
import { expectSidebarVisible, dismissToasts } from './helpers';

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
      page.getByRole('heading', { name: /メールテーブル/ })
    ).toBeVisible();
  });

  test('複数のテーブルに遷移できる', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-emails')();
    await expect(
      page.getByRole('heading', { name: /メールテーブル/ })
    ).toBeVisible({ timeout: 5000 });

    const tables = [
      { id: 'table-orders', heading: '注文テーブル' },
      { id: 'table-items', heading: '商品アイテムテーブル' },
      { id: 'table-images', heading: '画像テーブル' },
      { id: 'table-deliveries', heading: '配送情報テーブル' },
      { id: 'table-htmls', heading: 'HTML本文テーブル' },
      { id: 'table-order-emails', heading: '注文-メールテーブル' },
      { id: 'table-order-htmls', heading: '注文-HTMLテーブル' },
      { id: 'table-shop-settings', heading: '店舗設定テーブル' },
      { id: 'table-product-master', heading: '商品マスタテーブル' },
      { id: 'table-item-overrides', heading: 'アイテム上書きテーブル' },
      { id: 'table-order-overrides', heading: '注文上書きテーブル' },
      { id: 'table-excluded-items', heading: '除外アイテムテーブル' },
      { id: 'table-excluded-orders', heading: '除外注文テーブル' },
    ] as const;
    for (const table of tables) {
      await page.getByTestId(table.id).click();
      await expect(
        page.getByRole('heading', {
          name: new RegExp(table.heading),
        })
      ).toBeVisible({ timeout: 5000 });
    }
  });

  test('EmailsからOrdersテーブルに遷移できる', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-emails')();
    await expect(
      page.getByRole('heading', { name: /メールテーブル/ })
    ).toBeVisible({ timeout: 5000 });
    await page.getByTestId('table-orders').click();
    await expect(
      page.getByRole('heading', { name: /注文テーブル/ })
    ).toBeVisible({ timeout: 5000 });
  });

  test('テーブル画面で読み込み中または結果が表示される', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-emails')();
    await expect(
      page.getByRole('heading', { name: /メールテーブル/ })
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
      page.getByRole('heading', { name: /メールテーブル/ })
    ).toBeVisible();
    await dismissToasts(page);
    const refreshButton = page.getByRole('button', { name: '更新' });
    await expect(refreshButton).toBeVisible({ timeout: 10000 });
    await refreshButton.click();
  });

  test('ページネーションで次へをクリックできる', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-shop-settings')();
    await expect(
      page.getByRole('heading', { name: /店舗設定テーブル/ })
    ).toBeVisible({ timeout: 10000 });
    await dismissToasts(page);
    await expect(page.getByText(/55件/)).toBeVisible();
    const nextButton = page.getByRole('button', { name: /次へ/ });
    await expect(nextButton).toBeEnabled();
    await nextButton.click();
    await expect(page.getByText(/ページ 2/)).toBeVisible();
  });

  test('前へ・次へボタンが表示される', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-emails')();
    await expect(
      page.getByRole('heading', { name: /メールテーブル/ })
    ).toBeVisible();
    await expect(page.getByRole('button', { name: /前へ/ })).toBeVisible({
      timeout: 10000,
    });
    await expect(page.getByRole('button', { name: /次へ/ })).toBeVisible();
  });

  test('セルをクリックすると詳細ダイアログが表示される', async ({ page }) => {
    await expandTablesAndNavigate(page, 'table-shop-settings')();
    await expect(
      page.getByRole('heading', { name: /店舗設定テーブル/ })
    ).toBeVisible({ timeout: 10000 });
    await dismissToasts(page);
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
