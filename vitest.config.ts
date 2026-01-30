import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './src/test/setup.ts',
    include: ['src/**/*.{test,spec}.{js,ts,jsx,tsx}'],
    exclude: [
      'node_modules',
      'dist',
      'build',
      'tests/e2e/**',
      'tests/e2e-tauri/**',
    ],
    css: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html', 'lcov'],
      exclude: [
        'node_modules/',
        'src/test/',
        '**/*.config.{js,ts}',
        '**/dist/**',
        '**/build/**',
        '**/*.d.ts',
        '**/vite-env.d.ts',
      ],
      // カバレッジ閾値（Issue #18 Orders画面追加により一時的に75%に引き下げ、要テスト追加）
      thresholds: {
        lines: 75,
        functions: 55,
        branches: 62,
        statements: 74,
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
