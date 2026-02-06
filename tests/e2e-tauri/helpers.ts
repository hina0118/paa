/**
 * WebdriverIO 用ヘルパー（Tauri E2E）
 *
 * Playwright の helpers.ts と同様の機能を WebdriverIO API で提供
 */

import { $, $$, expect } from '@wdio/globals';

/**
 * サイドバーから指定の画面に遷移する
 */
export async function navigateToScreen(screenName: string) {
  const buttons = await $$('button');
  const btn = await buttons.find(
    async (el) => (await el.getText()) === screenName
  );
  if (!btn) throw new Error(`Button "${screenName}" not found`);
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
  const tablesBtn = await $('button*=Tables');
  await tablesBtn.waitForDisplayed({ timeout: 5000 });
  const text = await tablesBtn.getText();
  if (text.includes('▶')) {
    await tablesBtn.click();
    // 展開アニメーション待ち
    const emailsBtn = await $('button=Emails');
    await emailsBtn.waitForDisplayed({ timeout: 3000 });
  }
}

/**
 * Tables セクション内のサブメニューをクリックする
 * 注: トップレベルに同名の「Orders」があるため、Tables 展開後の ul.ml-4 内のボタンのみ対象にする
 */
export async function navigateToTable(tableName: string) {
  await expandTablesSection();
  // Tables 展開後のサブメニューは ul.ml-4 内にある（トップレベルと区別）
  const tableButtons = await $$('aside ul.ml-4 button');
  const btn = await tableButtons.find(
    async (el) => (await el.getText()) === tableName
  );
  if (!btn) throw new Error(`Table button "${tableName}" not found`);
  await btn.click();
}
