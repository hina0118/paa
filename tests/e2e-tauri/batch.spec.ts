/**
 * Tauri アプリのバッチ処理画面 E2E テスト
 *
 * PAA_E2E_MOCK=1 により Gmail/Gemini はモック化されている
 */

import { $, expect } from '@wdio/globals';
import {
  navigateToScreen,
  expectScreenTitle,
  expectSidebarVisible,
} from './helpers';

describe('バッチ処理 (Tauri)', () => {
  before(async () => {
    await expectSidebarVisible();
    await navigateToScreen('Batch');
    await expectScreenTitle('バッチ処理');
  });

  it('Gmail同期セクションが表示される', async () => {
    const heading = await $('h2*=Gmail同期');
    await expect(heading).toBeDisplayed();
  });

  it('メールパースセクションが表示される', async () => {
    const heading = await $('h2*=メールパース');
    await expect(heading).toBeDisplayed();
  });

  it('同期を開始ボタンがクリックできる', async () => {
    const startBtn = await $('button*=同期を開始');
    await startBtn.waitForDisplayed({ timeout: 5000 });
    await startBtn.click();
    // モックにより「新規メッセージなし」で即完了する想定
    await expect(startBtn).toBeDisplayed();
  });

  it('パースを開始ボタンをクリックすると確認ダイアログが表示される', async () => {
    const parseBtn = await $('button*=パースを開始');
    await parseBtn.waitForDisplayed({ timeout: 5000 });
    await parseBtn.click();

    const dialog = await $('[role="dialog"]');
    await expect(dialog).toBeDisplayed({ wait: 5000 });
    const cancelBtn = await $('button=キャンセル');
    await cancelBtn.click();
  });

  it('商品名パースセクションが表示される', async () => {
    const heading = await $('h2*=商品名パース');
    await expect(heading).toBeDisplayed();
  });
});
