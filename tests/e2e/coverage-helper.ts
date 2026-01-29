import * as fs from 'fs';
import * as path from 'path';

/**
 * カバレッジデータをJSONファイルに保存
 */
export function saveCoverageData(
  coverageData: Array<{
    url: string;
    text: string;
    functions: Array<{
      functionName: string;
      ranges: Array<{
        startOffset: number;
        endOffset: number;
        count: number;
      }>;
    }>;
  }>,
  outputPath: string = 'coverage-e2e/coverage.json'
) {
  const coverageDir = path.dirname(outputPath);
  if (!fs.existsSync(coverageDir)) {
    fs.mkdirSync(coverageDir, { recursive: true });
  }

  fs.writeFileSync(outputPath, JSON.stringify(coverageData, null, 2));
  console.log(`📊 カバレッジデータを保存しました: ${outputPath}`);
}

/**
 * カバレッジサマリーを生成
 */
export function generateCoverageSummary(
  coverageData: Array<{
    url: string;
    text: string;
    functions: Array<{
      functionName: string;
      ranges: Array<{
        startOffset: number;
        endOffset: number;
        count: number;
      }>;
    }>;
  }>
): {
  totalFiles: number;
  totalFunctions: number;
  coveredFunctions: number;
  coveragePercentage: number;
} {
  let totalFunctions = 0;
  let coveredFunctions = 0;

  coverageData.forEach((file) => {
    file.functions.forEach((func) => {
      totalFunctions++;
      // 少なくとも1つのrangeが実行された場合、カバーされているとみなす
      const hasCoverage = func.ranges.some((range) => range.count > 0);
      if (hasCoverage) {
        coveredFunctions++;
      }
    });
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
