/**
 * Tauri アプリ起動時の設定画面 E2E テスト（Rust コマンド invoke が動く）
 */

describe('設定画面 (Tauri)', () => {
  before(async () => {
    const buttons = await $$('button');
    const settingsBtn = await buttons.find(
      async (el) => (await el.getText()) === 'Settings'
    );
    if (!settingsBtn) throw new Error('Settings button not found');
    await settingsBtn.click();
    const heading = await $('h1=設定');
    await expect(heading).toBeDisplayed({ wait: 10000 });
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
});
