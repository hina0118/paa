/**
 * Tauri アプリ起動時の設定画面 E2E テスト（Rust コマンド invoke が動く）
 */

import { $, expect } from '@wdio/globals';
import {
  navigateToScreen,
  expectScreenTitle,
  expectSidebarVisible,
} from './helpers';

describe('設定画面 (Tauri)', () => {
  before(async () => {
    await expectSidebarVisible();
    await navigateToScreen('Settings');
    await expectScreenTitle('設定');
  });

  it('バッチサイズを変更して保存できる（Tauri API 経由）', async () => {
    const batchInput = await $('#batch-size');
    await batchInput.waitForDisplayed({ timeout: 10000 });
    const current = await batchInput.getValue();
    const newVal = current === '10' ? '20' : '10';
    await batchInput.clearValue();
    await batchInput.setValue(newVal);

    const saveBtn = await $('button=保存');
    await saveBtn.click();

    // Tauri アプリなので invoke が動き、成功メッセージが出る（テキストで識別）
    const success = await $('div*=バッチサイズを更新しました');
    await expect(success).toBeDisplayed({ wait: 10000 });
    await expect(success).toHaveTextContaining('バッチサイズを更新しました');
  });

  it('パースバッチサイズを変更して保存できる', async () => {
    const parseBatchInput = await $('#parse-batch-size');
    await parseBatchInput.waitForDisplayed({ timeout: 10000 });
    const current = await parseBatchInput.getValue();
    const newVal = current === '50' ? '100' : '50';
    await parseBatchInput.clearValue();
    await parseBatchInput.setValue(newVal);

    const saveBtn = await $('button[aria-label="パースバッチサイズを保存"]');
    await saveBtn.waitForDisplayed({ timeout: 5000 });
    await saveBtn.click();

    const success = await $('div*=パースバッチサイズを更新しました');
    await expect(success).toBeDisplayed({ wait: 10000 });
  });

  it('Geminiバッチサイズを変更して保存できる', async () => {
    const geminiInput = await $('#gemini-batch-size');
    await geminiInput.waitForDisplayed({ timeout: 10000 });
    const current = await geminiInput.getValue();
    const newVal = current === '10' ? '20' : '10';
    await geminiInput.clearValue();
    await geminiInput.setValue(newVal);

    const saveBtn = await $(
      'button[aria-label="商品名パースのバッチサイズを保存"]'
    );
    await saveBtn.waitForDisplayed({ timeout: 5000 });
    await saveBtn.click();

    const success = await $('div*=商品名パースのバッチサイズを更新しました');
    await expect(success).toBeDisplayed({ wait: 10000 });
  });
});
