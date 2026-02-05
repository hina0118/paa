/**
 * Tauri アプリの Tables 画面 E2E テスト
 */

import { $, expect } from '@wdio/globals';
import { navigateToTable, expectSidebarVisible } from './helpers';

describe('Tables (Tauri)', () => {
  it('Orders テーブルに遷移して表示される', async () => {
    await expectSidebarVisible();
    await navigateToTable('Orders');
    const heading = await $('h1*=Orders');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });
});
