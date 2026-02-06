/**
 * Tauri アプリのログ画面 E2E テスト
 */

import { $, expect } from '@wdio/globals';
import { navigateToScreen, expectSidebarVisible } from './helpers';

describe('ログ (Tauri)', () => {
  before(async () => {
    await expectSidebarVisible();
    await navigateToScreen('Logs');
    const heading = await $('h1');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });

  it('ログ画面のタイトルが表示される', async () => {
    const heading = await $('h1');
    await expect(heading).toBeDisplayed();
  });

  it('更新ボタンが表示される', async () => {
    const updateBtn = await $('button*=更新');
    await expect(updateBtn).toBeDisplayed({ wait: 5000 });
  });

  it('更新ボタンをクリックできる', async () => {
    const updateBtn = await $('button*=更新');
    await updateBtn.waitForDisplayed({ timeout: 5000 });
    await updateBtn.click();
    // Tauri invoke でログ取得
    await expect(updateBtn).toBeDisplayed();
  });
});
