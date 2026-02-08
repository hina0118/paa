import { defineConfig, devices } from '@playwright/test';

// NOTE:
// - Phase 1 ではまず「ブラウザE2Eテストの足場」を用意します。
// - 実際の Tauri アプリ起動連携（テスト専用ビルドやDB切り替えなど）は
//   Phase 2 以降で具体的なシナリオ実装時に詰めていきます。
// - 現状は `src-tauri/tauri.conf.json` の `devUrl` (http://localhost:1420)
//   を前提に、既にローカルで起動しているアプリに対してテストする形です。

export default defineConfig({
  testDir: './tests/e2e',
  timeout: 60_000,
  expect: {
    timeout: 5_000,
  },
  // CI ではカバレッジ収集のため並列を無効化（複数ワーカーが同一ファイルに書き込むと競合する）
  fullyParallel: !process.env.CI,
  // CI環境ではHTMLレポートも生成
  reporter: process.env.CI
    ? [['list'], ['html', { outputFolder: 'playwright-report' }]]
    : [['list']],
  globalTeardown: './tests/e2e/coverage-reporter.ts',
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  // テスト実行時に自動で Vite 開発サーバを起動する
  // （ローカルでは既存サーバがあればそれを再利用し、CI では毎回起動）
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:1420',
    reuseExistingServer: process.env.CI ? false : true,
    timeout: 120_000,
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  // カバレッジ設定（Chromiumのみ対応）
  // 注意: Playwrightの組み込みカバレッジはJS/CSSカバレッジのみで、
  // より詳細なカバレッジ（行・分岐・関数）が必要な場合は
  // vite-plugin-istanbul などの追加設定が必要です
});
