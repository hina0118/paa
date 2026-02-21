/**
 * Tauri アプリの Tables 画面 E2E テスト
 */

import { $, expect } from '@wdio/globals';
import { navigateToTable, expectSidebarVisible } from './helpers';

describe('Tables (Tauri)', () => {
  it('Orders テーブルに遷移して表示される', async () => {
    await expectSidebarVisible();
    await navigateToTable('table-orders');
    // TableViewer の title は "注文テーブル"。h1 で表示される
    const heading = await $('h1*=注文テーブル');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });
});
