import { test as base } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

// カバレッジデータを保存するファイル
const coverageFile = path.join(
  process.cwd(),
  'coverage-e2e',
  'coverage-data.json'
);

// カバレッジディレクトリを作成
const coverageDir = path.dirname(coverageFile);
if (!fs.existsSync(coverageDir)) {
  fs.mkdirSync(coverageDir, { recursive: true });
}

// カバレッジデータを収集する拡張テスト（追加フィクスチャなし）
export const test = base.extend<Record<string, never>>({});

// 全テストでカバレッジを収集
test.beforeEach(async ({ page, browserName }) => {
  if (browserName === 'chromium') {
    await page.coverage.startJSCoverage();
  }
});

test.afterEach(async ({ page, browserName }) => {
  if (browserName === 'chromium') {
    try {
      const coverage = await page.coverage.stopJSCoverage();
      if (coverage && coverage.length > 0) {
        // カバレッジデータをファイルに保存
        let existingData: any[] = [];
        if (fs.existsSync(coverageFile)) {
          try {
            existingData = JSON.parse(fs.readFileSync(coverageFile, 'utf-8'));
          } catch {
            existingData = [];
          }
        }

        // URLで重複をチェックしてマージ
        const urlMap = new Map<string, any>();
        existingData.forEach((item) => {
          urlMap.set(item.url, item);
        });

        coverage.forEach((item) => {
          if (!urlMap.has(item.url)) {
            urlMap.set(item.url, item);
          }
        });

        fs.writeFileSync(
          coverageFile,
          JSON.stringify(Array.from(urlMap.values()), null, 2)
        );
      }
    } catch (error) {
      // カバレッジ収集エラーは無視（テストは続行）
      console.warn('カバレッジ収集エラー:', error);
    }
  }
});

export { expect } from '@playwright/test';
