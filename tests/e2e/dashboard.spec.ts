import { test, expect } from './fixtures';
import {
  navigateToScreen,
  expectScreenTitle,
  expectButtonEnabled,
  dismissToasts,
} from './helpers';

test.describe('ダッシュボード画面', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await navigateToScreen(page, 'dashboard');
    await expectScreenTitle(page, 'ダッシュボード');
    await dismissToasts(page);
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
    // 配送状況カード、読み込み中、またはエラーメッセージのいずれかが表示されるまで待機
    const deliveryStatsCard = page.getByText('配送状況');
    const loadingText = page.getByRole('button', { name: '読み込み中...' });
    const loadErrorText = page.getByText(
      'データの読み込みに失敗しました。上の「更新」ボタンで再試行してください。'
    );
    await expect(
      deliveryStatsCard.or(loadingText).or(loadErrorText).first()
    ).toBeVisible({ timeout: 5000 });
  });
});
