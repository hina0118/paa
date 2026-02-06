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
    // ダッシュボードのボタンは「更新」または「読み込み中...」
    const loadBtn = await $('button*=更新');
    await expect(loadBtn).toBeDisplayed({ wait: 5000 });
  });

  it('統計を読み込みボタンをクリックできる', async () => {
    const loadBtn = await $('button*=更新');
    await loadBtn.waitForDisplayed({ timeout: 5000 });
    await loadBtn.click();
    // クリック後は「読み込み中...」または「更新」に戻る
    await expect(loadBtn).toBeDisplayed();
  });
});
