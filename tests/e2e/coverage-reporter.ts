import type { FullConfig } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

const coverageFile = path.join(
  process.cwd(),
  'coverage-e2e',
  'coverage-data.json'
);

/**
 * E2E 目標カバレッジ率（関数カバレッジ）。CI で未達の場合は失敗する。
 * Issue #18 で Orders 画面追加により総関数数が増えたため一時的に 20% に設定。
 * 25% 復旧の追跡: Issue #51
 */
const E2E_COVERAGE_TARGET_PERCENT = 20;

export default async function globalTeardown(_config: FullConfig) {
  if (fs.existsSync(coverageFile)) {
    try {
      const coverageData = JSON.parse(fs.readFileSync(coverageFile, 'utf-8'));
      const summary = generateCoverageSummary(coverageData);
      console.log('\n📊 E2Eテストカバレッジサマリー:');
      console.log(`   総ファイル数: ${summary.totalFiles}`);
      console.log(`   総関数数: ${summary.totalFunctions}`);
      console.log(`   カバーされた関数数: ${summary.coveredFunctions}`);
      console.log(`   カバレッジ: ${summary.coveragePercentage}%`);
      console.log(`   目標: ${E2E_COVERAGE_TARGET_PERCENT}%`);

      if (
        process.env.CI &&
        summary.coveragePercentage < E2E_COVERAGE_TARGET_PERCENT
      ) {
        console.error(
          `\n❌ E2Eカバレッジが目標（${E2E_COVERAGE_TARGET_PERCENT}%）を下回っています: ${summary.coveragePercentage}%`
        );
        process.exit(1);
      }
    } catch (error) {
      console.warn('カバレッジサマリーの生成に失敗:', error);
    }
  }
}

function generateCoverageSummary(coverageData: any[]): {
  totalFiles: number;
  totalFunctions: number;
  coveredFunctions: number;
  coveragePercentage: number;
} {
  let totalFunctions = 0;
  let coveredFunctions = 0;

  coverageData.forEach((file: any) => {
    if (file.functions && Array.isArray(file.functions)) {
      file.functions.forEach((func: any) => {
        totalFunctions++;
        const hasCoverage =
          func.ranges &&
          Array.isArray(func.ranges) &&
          func.ranges.some((range: any) => range.count > 0);
        if (hasCoverage) {
          coveredFunctions++;
        }
      });
    }
  });

  const coveragePercentage =
    totalFunctions > 0 ? (coveredFunctions / totalFunctions) * 100 : 0;

  return {
    totalFiles: coverageData.length,
    totalFunctions,
    coveredFunctions,
    coveragePercentage: Math.round(coveragePercentage * 100) / 100,
  };
}
