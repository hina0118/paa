import { defineConfig, devices } from '@playwright/test';

// Rustカバレッジ計測用のPlaywright設定
// この設定では、Tauriアプリをカバレッジ計測モードで起動します

export default defineConfig({
  testDir: './tests/e2e',
  timeout: 60_000,
  expect: {
    timeout: 5_000,
  },
  fullyParallel: false, // カバレッジ計測時は並列実行を無効化
  reporter: [['list']],
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  // Tauriアプリをカバレッジ計測モードで起動
  // 注意: この設定を使用する場合は、事前にRUSTFLAGSとLLVM_PROFILE_FILEを設定してください
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:1420',
    reuseExistingServer: process.env.CI ? false : true,
    timeout: 120_000,
    env: {
      // Rustカバレッジ計測用の環境変数
      LLVM_PROFILE_FILE:
        process.env.LLVM_PROFILE_FILE || 'src-tauri/coverage-e2e.profraw',
      RUSTFLAGS: process.env.RUSTFLAGS || '-Cinstrument-coverage',
    },
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
