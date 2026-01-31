import { test, expect } from './fixtures';
import type { Page } from '@playwright/test';
import { expectSidebarVisible } from './helpers';

function expandTablesAndNavigate(page: Page, tableName: string) {
  return async () => {
    const tablesButton = page.getByRole('button', { name: /Tables/ });
    await tablesButton.click();
    const tablesSection = page.locator('ul').filter({
      has: page.getByRole('button', { name: 'Emails' }),
    });
    const tableButton = tablesSection.getByRole('button', {
      name: tableName,
      exact: true,
    });
    await tableButton.click();
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
    await expandTablesAndNavigate(page, 'Emails')();
    await expect(
      page.getByRole('heading', { name: /Emails テーブル/ })
    ).toBeVisible();
  });

  test('複数のテーブルに遷移できる', async ({ page }) => {
    await expandTablesAndNavigate(page, 'Emails')();
    await expect(
      page.getByRole('heading', { name: /Emails テーブル/ })
    ).toBeVisible({ timeout: 5000 });

    const tablesSection = page.locator('ul').filter({
      has: page.getByRole('button', { name: 'Emails' }),
    });
    const tables = [
      'Orders',
      'Items',
      'Images',
      'Deliveries',
      'HTMLs',
      'Order-Emails',
      'Order-HTMLs',
      'Shop Settings',
      'Sync Metadata',
      'Window Settings',
      'Parse Metadata',
    ] as const;
    for (const tableName of tables) {
      await tablesSection
        .getByRole('button', { name: tableName, exact: true })
        .click();
      await expect(
        page.getByRole('heading', { name: new RegExp(`${tableName} テーブル`) })
      ).toBeVisible({ timeout: 5000 });
    }
  });

  test('EmailsからOrdersテーブルに遷移できる', async ({ page }) => {
    await expandTablesAndNavigate(page, 'Emails')();
    await expect(
      page.getByRole('heading', { name: /Emails テーブル/ })
    ).toBeVisible({ timeout: 5000 });
    const tablesSection = page.locator('ul').filter({
      has: page.getByRole('button', { name: 'Emails' }),
    });
    await tablesSection
      .getByRole('button', { name: 'Orders', exact: true })
      .click();
    await expect(
      page.getByRole('heading', { name: /Orders テーブル/ })
    ).toBeVisible({ timeout: 5000 });
  });

  test('テーブル画面で読み込み中または結果が表示される', async ({ page }) => {
    await expandTablesAndNavigate(page, 'Emails')();
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
    await expandTablesAndNavigate(page, 'Emails')();
    await expect(
      page.getByRole('heading', { name: /Emails テーブル/ })
    ).toBeVisible();
    const refreshButton = page.getByRole('button', { name: '更新' });
    await expect(refreshButton).toBeVisible({ timeout: 10000 });
    await refreshButton.click();
  });

  test('ページネーションで次へをクリックできる', async ({ page }) => {
    await expandTablesAndNavigate(page, 'Shop Settings')();
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
    await expandTablesAndNavigate(page, 'Emails')();
    await expect(
      page.getByRole('heading', { name: /Emails テーブル/ })
    ).toBeVisible();
    await expect(page.getByRole('button', { name: /前へ/ })).toBeVisible({
      timeout: 10000,
    });
    await expect(page.getByRole('button', { name: /次へ/ })).toBeVisible();
  });

  test('セルをクリックすると詳細ダイアログが表示される', async ({ page }) => {
    await expandTablesAndNavigate(page, 'Shop Settings')();
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
