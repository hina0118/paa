/**
 * WebdriverIO 用ヘルパー（Tauri E2E）
 *
 * Playwright の helpers.ts と同様の機能を WebdriverIO API で提供
 */

import { $, expect } from '@wdio/globals';

/**
 * サイドバーから指定の画面に遷移する
 * @param screenId - サイドバーボタンの data-testid 値（例: 'dashboard', 'shop-settings'）
 */
export async function navigateToScreen(screenId: string) {
  const btn = await $(`[data-testid="${screenId}"]`);
  await btn.waitForDisplayed({ timeout: 5000 });
  await btn.click();
}

/**
 * 現在の画面が指定のタイトルであることを確認する
 */
export async function expectScreenTitle(title: string) {
  const heading = await $(`h1=${title}`);
  await expect(heading).toBeDisplayed({ wait: 10000 });
}

/**
 * サイドバーが表示されていることを確認する
 * 注意: *=PAA は partial link text で <a> のみ対象。PAA は <h2> 内にあるため h2=PAA を使用
 */
export async function expectSidebarVisible() {
  const sidebar = await $('aside');
  await expect(sidebar).toBeDisplayed();
  const paa = await $('h2*=PAA');
  await expect(paa).toBeDisplayed();
}

/**
 * Tables セクションを展開する
 */
export async function expandTablesSection() {
  const tablesBtn = await $('[data-testid="tables-section-toggle"]');
  await tablesBtn.waitForDisplayed({ timeout: 5000 });
  const text = await tablesBtn.getText();
  if (text.includes('▶')) {
    await tablesBtn.click();
    // 展開アニメーション待ち
    const emailsBtn = await $('[data-testid="table-emails"]');
    await emailsBtn.waitForDisplayed({ timeout: 3000 });
  }
}

/**
 * Tables セクション内のサブメニューをクリックする
 * @param tableId - テーブルボタンの data-testid 値（例: 'table-orders'）
 */
export async function navigateToTable(tableId: string) {
  await expandTablesSection();
  const btn = await $(`[data-testid="${tableId}"]`);
  await btn.waitForDisplayed({ timeout: 3000 });
  await btn.click();
}

/**
 * 表示中のSonnerトーストがすべて消えるまで待機する
 * クリック操作の前に呼び出すことでトーストによるブロックを防ぐ
 */
export async function dismissToasts() {
  while (true) {
    const toast = await $('[data-sonner-toast]');
    const exists = await toast.isExisting();
    if (!exists) return;
    const visible = await toast.isDisplayed();
    if (!visible) return;
    await toast.waitForDisplayed({ reverse: true, timeout: 10000 });
  }
}
