import { Page, expect } from '@playwright/test';

/**
 * サイドバーから指定の画面に遷移する
 */
export async function navigateToScreen(page: Page, screenName: string) {
  // "Settings"と"Shop Settings"が両方存在するため、exact matchを使用
  const button = page.getByRole('button', { name: screenName, exact: true });
  await button.click();
}

/**
 * 現在の画面が指定の画面であることを確認する
 */
export async function expectScreenTitle(page: Page, title: string) {
  await expect(
    page.getByRole('heading', { name: title, level: 1 })
  ).toBeVisible();
}

/**
 * サイドバーが表示されていることを確認する
 */
export async function expectSidebarVisible(page: Page) {
  await expect(page.getByRole('complementary')).toBeVisible();
  await expect(page.getByText('PAA')).toBeVisible();
}

/**
 * 成功メッセージが表示されることを確認する
 */
export async function expectSuccessMessage(page: Page, message?: string) {
  // 成功メッセージのコンテナを待機（bg-green-50クラスを持つ要素）
  const successElement = page.locator('.bg-green-50').first();
  await expect(successElement).toBeVisible({ timeout: 10000 });
  if (message) {
    await expect(successElement).toContainText(message, { timeout: 5000 });
  }
}

/**
 * エラーメッセージが表示されることを確認する
 */
export async function expectErrorMessage(page: Page, message?: string) {
  // エラーメッセージのコンテナを待機（bg-red-50クラスを持つ要素）
  const errorElement = page.locator('.bg-red-50').first();
  await expect(errorElement).toBeVisible({ timeout: 10000 });
  if (message) {
    await expect(errorElement).toContainText(message, { timeout: 5000 });
  }
}

/**
 * ボタンが無効化されていることを確認する
 */
export async function expectButtonDisabled(page: Page, buttonName: string) {
  const button = page.getByRole('button', { name: buttonName });
  await expect(button).toBeDisabled();
}

/**
 * ボタンが有効化されていることを確認する
 */
export async function expectButtonEnabled(page: Page, buttonName: string) {
  const button = page.getByRole('button', { name: buttonName });
  await expect(button).toBeEnabled();
}

/**
 * 入力フィールドに値を入力する
 */
export async function fillInput(
  page: Page,
  label: string | { id?: string; placeholder?: string },
  value: string
) {
  if (typeof label === 'string') {
    const input = page.getByLabel(label);
    await input.fill(value);
  } else {
    let input;
    if (label.id) {
      input = page.locator(`#${label.id}`);
    } else if (label.placeholder) {
      input = page.getByPlaceholder(label.placeholder);
    } else {
      throw new Error('label must have id or placeholder');
    }
    await input.fill(value);
  }
}

/**
 * カードが表示されていることを確認する
 */
export async function expectCardVisible(page: Page, title: string) {
  await expect(
    page.getByRole('heading', { name: title, level: 3 })
  ).toBeVisible();
}
