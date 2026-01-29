/**
 * Tauri アプリ起動時のナビゲーション E2E テスト
 */

describe('ナビゲーション (Tauri)', () => {
  it('サイドバーが表示される', async () => {
    const sidebar = await $('aside');
    await expect(sidebar).toBeDisplayed();
    const paa = await $('*=PAA');
    await expect(paa).toBeDisplayed();
  });

  it('Settings 画面に遷移できる', async () => {
    const buttons = await $$('button');
    const settingsBtn = await buttons.find(
      async (el) => (await el.getText()) === 'Settings'
    );
    if (!settingsBtn) throw new Error('Settings button not found');
    await settingsBtn.click();
    const heading = await $('h1=設定');
    await expect(heading).toBeDisplayed({ wait: 10000 });
  });

  it('設定画面で同期設定・パース設定カードが表示される', async () => {
    const syncHeading = await $('h3=同期設定');
    await expect(syncHeading).toBeDisplayed();
    const parseHeading = await $('h3=パース設定');
    await expect(parseHeading).toBeDisplayed();
  });
});
