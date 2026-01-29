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

        // 同一URLのカバレッジ ranges をマージするヘルパー
        const mergeCoverageItems = (existingItem: any, newItem: any): any => {
          const allRanges = [
            ...(existingItem.ranges || []),
            ...(newItem.ranges || []),
          ];
          if (allRanges.length === 0) {
            return existingItem;
          }
          allRanges.sort((a, b) => (a.startOffset ?? 0) - (b.startOffset ?? 0));
          const mergedRanges: any[] = [];
          let current = { ...allRanges[0] };
          for (let i = 1; i < allRanges.length; i++) {
            const next = allRanges[i];
            const currentEnd = current.endOffset ?? 0;
            const nextStart = next.startOffset ?? 0;
            const nextEnd = next.endOffset ?? 0;
            if (nextStart <= currentEnd) {
              current.endOffset = Math.max(currentEnd, nextEnd);
              if (current.count !== undefined || next.count !== undefined) {
                current.count = Math.max(current.count ?? 0, next.count ?? 0);
              }
            } else {
              mergedRanges.push(current);
              current = { ...next };
            }
          }
          mergedRanges.push(current);
          return { ...existingItem, ranges: mergedRanges };
        };

        const urlMap = new Map<string, any>();
        existingData.forEach((item) => {
          urlMap.set(item.url, item);
        });

        coverage.forEach((item) => {
          const existingItem = urlMap.get(item.url);
          if (existingItem) {
            urlMap.set(item.url, mergeCoverageItems(existingItem, item));
          } else {
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
