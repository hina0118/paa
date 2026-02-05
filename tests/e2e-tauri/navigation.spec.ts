/**
 * Tauri アプリ起動時のナビゲーション E2E テスト
 */

import { $, $$, expect } from '@wdio/globals';
import {
  navigateToScreen,
  expectScreenTitle,
  expectSidebarVisible,
  expandTablesSection,
  navigateToTable,
} from './helpers';

describe('ナビゲーション (Tauri)', () => {
  it('サイドバーが表示される', async () => {
    await expectSidebarVisible();
    const buttons = await $$('button');
    const texts = await Promise.all(buttons.map((b) => b.getText()));
    expect(texts).toContain('Dashboard');
    expect(texts).toContain('Orders');
    expect(texts).toContain('Batch');
    expect(texts).toContain('Logs');
    expect(texts).toContain('Shop Settings');
    expect(texts).toContain('Settings');
  });

  it('Dashboard 画面に遷移できる', async () => {
    await navigateToScreen('Dashboard');
    await expectScreenTitle('ダッシュボード');
  });

  it('Orders 画面に遷移できる', async () => {
    await navigateToScreen('Orders');
    await expectScreenTitle('商品一覧');
  });

  it('Batch 画面に遷移できる', async () => {
    await navigateToScreen('Batch');
    await expectScreenTitle('バッチ処理');
  });

  it('Logs 画面に遷移できる', async () => {
    await navigateToScreen('Logs');
    const heading = await $('h1');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });

  it('Shop Settings 画面に遷移できる', async () => {
    await navigateToScreen('Shop Settings');
    const heading = await $('h1');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });

  it('API Keys 画面に遷移できる', async () => {
    await navigateToScreen('API Keys');
    const heading = await $('h1');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });

  it('Settings 画面に遷移できる', async () => {
    await navigateToScreen('Settings');
    await expectScreenTitle('設定');
  });

  it('設定画面で同期設定・パース設定カードが表示される', async () => {
    await navigateToScreen('Settings');
    const syncHeading = await $('h3=同期設定');
    await expect(syncHeading).toBeDisplayed();
    const parseHeading = await $('h3=パース設定');
    await expect(parseHeading).toBeDisplayed();
  });

  it('Tables セクションを展開して Emails に遷移できる', async () => {
    await expandTablesSection();
    const emailsBtn = await $('button=Emails');
    await expect(emailsBtn).toBeDisplayed();
    await emailsBtn.click();
    const heading = await $('h1');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });

  it('Tables セクションを展開して閉じることができる', async () => {
    await expandTablesSection();
    const emailsBtn = await $('button=Emails');
    await expect(emailsBtn).toBeDisplayed();

    const tablesBtn = await $('button*=Tables');
    await tablesBtn.click();
    // 閉じた後は Tables ボタンに ▶ が表示される（折りたたみ状態）
    const tablesText = await tablesBtn.getText();
    expect(tablesText).toContain('▶');
  });
});
