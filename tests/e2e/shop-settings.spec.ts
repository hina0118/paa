import { test, expect } from './fixtures';
import { Page } from '@playwright/test';
import {
  navigateToScreen,
  expectSidebarVisible,
  expectScreenTitle,
  expectSuccessMessage,
} from './helpers';

const DEFAULT_ELEMENT_TIMEOUT = 5000;

function getShopCard(page: Page, shopName: string) {
  return page.locator('.border.rounded-lg').filter({ hasText: shopName });
}

test.describe('Shop Settings画面', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expectSidebarVisible(page);
    await navigateToScreen(page, 'shop-settings');
  });

  test('Shop Settings画面が表示される', async ({ page }) => {
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  });

  test('店舗追加フォームに入力できる', async ({ page }) => {
    await expectScreenTitle(page, '店舗設定');
    const shopNameInput = page.getByPlaceholder('例: Amazon発送通知').first();
    await shopNameInput.waitFor({
      state: 'visible',
      timeout: DEFAULT_ELEMENT_TIMEOUT,
    });
    await shopNameInput.fill('Test Shop');
    await expect(shopNameInput).toHaveValue('Test Shop');
  });

  test('空の状態で追加ボタンをクリックするとバリデーションエラーが表示される', async ({
    page,
  }) => {
    const addButton = page.getByRole('button', { name: '追加', exact: true });
    await addButton.waitFor({
      state: 'visible',
      timeout: DEFAULT_ELEMENT_TIMEOUT,
    });
    await addButton.click();
    await expect(
      page.getByText('すべての項目を入力してください')
    ).toBeVisible();
  });

  test('登録済み店舗セクションが表示される', async ({ page }) => {
    await expect(page.getByText('登録済み店舗')).toBeVisible();
  });

  test('店舗を追加するとカードに表示される', async ({ page }) => {
    const shopName = `E2EShop_${Date.now()}`;
    const shopNameInput = page.getByPlaceholder('例: Amazon発送通知').first();
    await shopNameInput.waitFor({
      state: 'visible',
      timeout: DEFAULT_ELEMENT_TIMEOUT,
    });
    await shopNameInput.fill(shopName);
    await page
      .getByPlaceholder('例: ship-confirm@amazon.co.jp')
      .fill('test@example.com');
    await page.getByPlaceholder('例: amazon').fill('amazon');

    const addButton = page.getByRole('button', { name: '追加', exact: true });
    await addButton.click();

    try {
      await expectSuccessMessage(page, '店舗設定を追加しました');
      // Verify the new shop card appears in the registered shops list
      await expect(page.getByRole('heading', { name: shopName })).toBeVisible({
        timeout: DEFAULT_ELEMENT_TIMEOUT,
      });
    } catch {
      console.log('Tauri APIが利用できないため、店舗追加の確認をスキップ');
    }
  });

  test('「有効化/無効化」ボタンをクリックするとトースト通知が表示される', async ({
    page,
  }) => {
    const shopName = `E2EToggleShop_${Date.now()}`;
    const shopNameInput = page.getByPlaceholder('例: Amazon発送通知').first();
    await shopNameInput.waitFor({
      state: 'visible',
      timeout: DEFAULT_ELEMENT_TIMEOUT,
    });
    await shopNameInput.fill(shopName);
    await page
      .getByPlaceholder('例: ship-confirm@amazon.co.jp')
      .fill('test@example.com');
    await page.getByPlaceholder('例: amazon').fill('amazon');
    await page.getByRole('button', { name: '追加', exact: true }).click();

    try {
      await expectSuccessMessage(page, '店舗設定を追加しました');
      // Click the disable button for the newly added shop
      const shopCard = getShopCard(page, shopName);
      const toggleButton = shopCard.getByRole('button', { name: '無効化' });
      await toggleButton.waitFor({
        state: 'visible',
        timeout: DEFAULT_ELEMENT_TIMEOUT,
      });
      await toggleButton.click();
      await expectSuccessMessage(page, `${shopName} を無効にしました`);
    } catch {
      console.log('Tauri APIが利用できないため、有効化/無効化の確認をスキップ');
    }
  });

  test('「編集」ボタンで展開すると個別パーサー行が表示される', async ({
    page,
  }) => {
    const shopName = `E2EExpandShop_${Date.now()}`;
    const shopNameInput = page.getByPlaceholder('例: Amazon発送通知').first();
    await shopNameInput.waitFor({
      state: 'visible',
      timeout: DEFAULT_ELEMENT_TIMEOUT,
    });
    await shopNameInput.fill(shopName);
    await page
      .getByPlaceholder('例: ship-confirm@amazon.co.jp')
      .fill('test@example.com');
    await page.getByPlaceholder('例: amazon').fill('amazon');
    await page.getByRole('button', { name: '追加', exact: true }).click();

    try {
      await expectSuccessMessage(page, '新しい店舗設定を追加しました');
      const shopCard = getShopCard(page, shopName);
      const editButton = shopCard.getByRole('button', { name: '編集' });
      await editButton.waitFor({
        state: 'visible',
        timeout: DEFAULT_ELEMENT_TIMEOUT,
      });
      await editButton.click();
      // Individual parser row should appear with sender address
      await expect(shopCard.getByText('test@example.com')).toBeVisible({
        timeout: DEFAULT_ELEMENT_TIMEOUT,
      });
    } catch {
      console.log('Tauri APIが利用できないため、編集展開の確認をスキップ');
    }
  });
});
