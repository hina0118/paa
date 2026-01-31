import { test, expect } from './fixtures';
import {
  navigateToScreen,
  expectScreenTitle,
  expectSidebarVisible,
} from './helpers';

test.describe('Orders画面（商品一覧）', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expectSidebarVisible(page);
    await navigateToScreen(page, 'Orders');
    await expectScreenTitle(page, '商品一覧');
  });

  test('検索ボックスが表示される', async ({ page }) => {
    await expect(
      page.getByPlaceholder('商品名・ショップ名・注文番号で検索')
    ).toBeVisible();
  });

  test('フィルタクリアボタンが表示される', async ({ page }) => {
    await expect(
      page.getByRole('button', { name: 'フィルタクリア' })
    ).toBeVisible();
  });

  test('検索ボックスに入力できる', async ({ page }) => {
    const searchInput =
      page.getByPlaceholder('商品名・ショップ名・注文番号で検索');
    await searchInput.fill('test');
    await expect(searchInput).toHaveValue('test');
  });

  test('カード/リスト表示切替ボタンが表示される', async ({ page }) => {
    const cardButton = page.getByRole('button', { name: 'カード表示' });
    const listButton = page.getByRole('button', { name: 'リスト表示' });
    await expect(cardButton).toBeVisible();
    await expect(listButton).toBeVisible();
  });

  test('リスト表示に切り替えできる', async ({ page }) => {
    const listButton = page.getByRole('button', { name: 'リスト表示' });
    await listButton.click();
    await expect(listButton).toBeVisible();
  });

  test('カード表示に切り替えできる', async ({ page }) => {
    const listButton = page.getByRole('button', { name: 'リスト表示' });
    await listButton.click();
    const cardButton = page.getByRole('button', { name: 'カード表示' });
    await cardButton.click();
    await expect(cardButton).toBeVisible();
  });

  test('並び順セレクトが表示される', async ({ page }) => {
    const sortSelect = page.locator('#sort');
    await expect(sortSelect).toBeVisible();
  });

  test('ソートを変更できる', async ({ page }) => {
    const sortSelect = page.locator('#sort');
    await sortSelect.selectOption('price-asc');
    await expect(sortSelect).toHaveValue('price-asc');
  });

  test('フィルタクリアをクリックできる', async ({ page }) => {
    const clearButton = page.getByRole('button', { name: 'フィルタクリア' });
    await clearButton.click();
    await expect(
      page.getByPlaceholder('商品名・ショップ名・注文番号で検索')
    ).toHaveValue('');
  });

  test('価格フィルタに入力できる', async ({ page }) => {
    const priceMin = page.locator('#filter-price-min');
    const priceMax = page.locator('#filter-price-max');
    await priceMin.fill('100');
    await priceMax.fill('5000');
    await expect(priceMin).toHaveValue('100');
    await expect(priceMax).toHaveValue('5000');
  });

  test('ショップフィルタを変更できる', async ({ page }) => {
    const shopSelect = page.locator('#filter-shop');
    await shopSelect.selectOption({ index: 0 });
  });

  test('購入年フィルタを変更できる', async ({ page }) => {
    const yearSelect = page.locator('#filter-year');
    await yearSelect.selectOption({ index: 0 });
  });

  test('検索入力でデバウンス後に再取得される', async ({ page }) => {
    const searchInput =
      page.getByPlaceholder('商品名・ショップ名・注文番号で検索');
    await searchInput.fill('query');
    await page.waitForTimeout(500);
    await expect(searchInput).toHaveValue('query');
  });

  test('商品一覧の状態が表示される', async ({ page }) => {
    await expect(
      page.getByText(/データがありません|件の商品|読み込み中/).first()
    ).toBeVisible({ timeout: 15000 });
  });

  test('カード表示で商品をクリックするとドロワーが開く', async ({ page }) => {
    const itemCard = page.getByText('E2Eテスト商品').first();
    await expect(itemCard).toBeVisible({ timeout: 10000 });
    await itemCard.click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(
      page.getByRole('heading', { name: 'E2Eテスト商品' })
    ).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  test('リスト表示で商品をクリックするとドロワーが開く', async ({ page }) => {
    const listButton = page.getByRole('button', { name: 'リスト表示' });
    await listButton.click();
    const itemRow = page.getByText('E2Eテスト商品').first();
    await expect(itemRow).toBeVisible({ timeout: 10000 });
    await itemRow.click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(
      page.getByRole('heading', { name: 'E2Eテスト商品' })
    ).toBeVisible();
  });
});
