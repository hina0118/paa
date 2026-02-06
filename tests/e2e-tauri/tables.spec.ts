/**
 * Tauri アプリの Tables 画面 E2E テスト
 */

import { $, expect } from '@wdio/globals';
import { navigateToTable, expectSidebarVisible } from './helpers';

describe('Tables (Tauri)', () => {
  it('Orders テーブルに遷移して表示される', async () => {
    await expectSidebarVisible();
    await navigateToTable('Orders');
    // TableViewer の title は "Orders テーブル"。h1 で表示される
    const heading = await $('h1*=Orders テーブル');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });
});
