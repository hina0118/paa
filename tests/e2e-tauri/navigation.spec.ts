/**
 * Tauri アプリ起動時のナビゲーション E2E テスト
 */

import { $, expect } from '@wdio/globals';
import {
  navigateToScreen,
  expectScreenTitle,
  expectSidebarVisible,
  expandTablesSection,
} from './helpers';

describe('ナビゲーション (Tauri)', () => {
  it('サイドバーが表示される', async () => {
    await expectSidebarVisible();
    for (const id of ['dashboard', 'orders', 'batch', 'logs', 'shop-settings', 'api-keys', 'settings']) {
      const btn = await $(`[data-testid="${id}"]`);
      await expect(btn).toBeDisplayed();
    }
  });

  it('Dashboard 画面に遷移できる', async () => {
    await navigateToScreen('dashboard');
    await expectScreenTitle('ダッシュボード');
  });

  it('Orders 画面に遷移できる', async () => {
    await navigateToScreen('orders');
    await expectScreenTitle('商品一覧');
  });

  it('Batch 画面に遷移できる', async () => {
    await navigateToScreen('batch');
    await expectScreenTitle('バッチ処理');
  });

  it('Logs 画面に遷移できる', async () => {
    await navigateToScreen('logs');
    const heading = await $('h1');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });

  it('Shop Settings 画面に遷移できる', async () => {
    await navigateToScreen('shop-settings');
    const heading = await $('h1');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });

  it('データのバックアップ画面に遷移できる', async () => {
    await navigateToScreen('backup');
    await expectScreenTitle('データのバックアップ');
  });

  it('API Keys 画面に遷移できる', async () => {
    await navigateToScreen('api-keys');
    const heading = await $('h1');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });

  it('Settings 画面に遷移できる', async () => {
    await navigateToScreen('settings');
    await expectScreenTitle('設定');
  });

  it('設定画面で同期設定・パース設定カードが表示される', async () => {
    await navigateToScreen('settings');
    const syncHeading = await $('h3=同期設定');
    await expect(syncHeading).toBeDisplayed();
    const parseHeading = await $('h3=パース設定');
    await expect(parseHeading).toBeDisplayed();
  });

  it('Tables セクションを展開して Emails に遷移できる', async () => {
    await expandTablesSection();
    const emailsBtn = await $('[data-testid="table-emails"]');
    await expect(emailsBtn).toBeDisplayed();
    await emailsBtn.click();
    const heading = await $('h1');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });

  it('Tables セクションを展開して閉じることができる', async () => {
    await expandTablesSection();
    const emailsBtn = await $('[data-testid="table-emails"]');
    await expect(emailsBtn).toBeDisplayed();

    const tablesBtn = await $('[data-testid="tables-section-toggle"]');
    await tablesBtn.click();
    // 閉じた後は Tables ボタンに ▶ が表示される（折りたたみ状態）
    const tablesText = await tablesBtn.getText();
    expect(tablesText).toContain('▶');
  });
});
