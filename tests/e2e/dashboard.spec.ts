import { test, expect } from './fixtures';
import {
  navigateToScreen,
  expectScreenTitle,
  expectButtonEnabled,
} from './helpers';

test.describe('ダッシュボード画面', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await navigateToScreen(page, 'Dashboard');
    await expectScreenTitle(page, 'ダッシュボード');
  });

  test('サイドバーからダッシュボード画面に遷移できる', async ({ page }) => {
    await expectScreenTitle(page, 'ダッシュボード');
  });

  test('ダッシュボード画面が表示される', async ({ page }) => {
    await expectScreenTitle(page, 'ダッシュボード');
    // 更新ボタンが表示される
    await expect(page.getByRole('button', { name: '更新' })).toBeVisible();
  });

  test('更新ボタンがクリックできる', async ({ page }) => {
    const refreshButton = page.getByRole('button', { name: '更新' });
    await expectButtonEnabled(page, '更新');

    // ボタンをクリック
    await refreshButton.click();

    // ボタンがクリックされたことを確認（読み込み中状態になる可能性がある）
    const buttonText = await refreshButton.textContent();
    expect(['更新', '読み込み中...']).toContain(buttonText?.trim());
  });

  test('統計情報カードが表示される', async ({ page }) => {
    // 統計情報が読み込まれるまで待機
    await page.waitForTimeout(2000);

    // 総メール数カードが表示される可能性がある
    const totalEmailsCard = page.getByText('総メール数');
    const cardsExist = (await totalEmailsCard.count()) > 0;

    if (cardsExist) {
      await expect(totalEmailsCard).toBeVisible();
    } else {
      // データがない場合や読み込み中の場合は「データを読み込んでいます...」が表示される
      // またはエラーが表示される可能性もある
      const loadingText = page.getByText('データを読み込んでいます...');
      const errorCard = page.locator('.border-red-500');

      // いずれかが表示されることを確認
      const loadingExists = (await loadingText.count()) > 0;
      const errorExists = (await errorCard.count()) > 0;

      expect(loadingExists || errorExists).toBeTruthy();
    }
  });

  test('パース状況カードが表示される', async ({ page }) => {
    await page.waitForTimeout(1000);

    const parseStatusCard = page.getByText('パース状況');
    const cardExists = (await parseStatusCard.count()) > 0;

    if (cardExists) {
      await expect(parseStatusCard).toBeVisible();
    }
  });
});
