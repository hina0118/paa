/**
 * WebdriverIO 用ヘルパー（Tauri E2E）
 *
 * Playwright の helpers.ts と同様の機能を WebdriverIO API で提供
 */

import { $, $$, expect } from '@wdio/globals';

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
 * 表示中のSonnerトーストが消えるまでベストエフォートで待機する（最大 MAX_ITERATIONS 回）
 * クリック操作の前に呼び出すことでトーストによるブロックを防ぐ。
 * 上限に達してもトーストが残っている場合は待機を諦め、後続のクリック操作側で成否を判定させる。
 */
export async function dismissToasts() {
  const MAX_ITERATIONS = 10;
  for (let _i = 0; _i < MAX_ITERATIONS; _i++) {
    const toasts = await $$('[data-sonner-toast]');
    if (toasts.length === 0) return;
    try {
      await toasts[0].waitForDisplayed({ reverse: true, timeout: 10000 });
    } catch (error) {
      // waitForDisplayed はタイムアウト時に例外を投げるが、このヘルパーでは
      // 「一定時間待っても消えない場合は待機を諦めて次の操作に進む」方針とする。
      if (error instanceof Error && error.name === 'WaitUntilTimeoutError') {
        return;
      }
      // タイムアウト以外のエラーは想定外なのでそのまま送出する
      throw error;
    }
  }
  // MAX_ITERATIONS 到達時もトーストが残っている可能性があるが、
  // ここでは待機を諦め、後続のクリック操作側で成否を判定させる。
}
