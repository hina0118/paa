/**
 * Tauri アプリのダッシュボード画面 E2E テスト
 */

import { $, expect } from '@wdio/globals';
import {
  navigateToScreen,
  expectScreenTitle,
  expectSidebarVisible,
} from './helpers';

describe('ダッシュボード (Tauri)', () => {
  before(async () => {
    await expectSidebarVisible();
    await navigateToScreen('Dashboard');
    await expectScreenTitle('ダッシュボード');
  });

  it('統計を読み込みボタンが表示される', async () => {
    const loadBtn = await $('button*=読み込み');
    await expect(loadBtn).toBeDisplayed({ wait: 5000 });
  });

  it('統計を読み込みボタンをクリックできる', async () => {
    const loadBtn = await $('button*=読み込み');
    await loadBtn.waitForDisplayed({ timeout: 5000 });
    await loadBtn.click();
    // 読み込み中または統計表示（Tauri invoke が動く）
    await expect(loadBtn).toBeDisplayed();
  });
});
