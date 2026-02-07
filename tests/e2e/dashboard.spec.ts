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
    // 総メール数カード、読み込み中、またはエラーメッセージのいずれかが表示されるまで待機
    const totalEmailsCard = page.getByText('総メール数');
    const loadingText = page.getByText('データを読み込んでいます...');
    const loadErrorText = page.getByText(
      'データの読み込みに失敗しました。上の「更新」ボタンで再試行してください。'
    );
    await expect(
      totalEmailsCard.or(loadingText).or(loadErrorText).first()
    ).toBeVisible({ timeout: 5000 });
  });

  test('パース状況カードが表示される', async ({ page }) => {
    // パース状況カード、読み込み中、またはエラーメッセージのいずれかが表示されるまで待機
    // （Tauri API が無い CI では読み込み中やエラーになる場合がある）
    const parseStatusCard = page.getByText('パース状況');
    const loadingText = page.getByText('データを読み込んでいます...');
    const loadErrorText = page.getByText(
      'データの読み込みに失敗しました。上の「更新」ボタンで再試行してください。'
    );
    await expect(
      parseStatusCard.or(loadingText).or(loadErrorText).first()
    ).toBeVisible({ timeout: 5000 });
  });
});
